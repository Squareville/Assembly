//! Low-Level reader for FDB files
//!
//! This module provides a struct which can be used to access
//! a FDB file in any order the user desires.

use std::io::{self, BufRead, Read, Seek, SeekFrom};

use super::parser;
use super::{
    file::*,
    parser::{ParseFDB, ParseLE},
};

use assembly_core::{
    nom::{Finish, IResult},
    reader::{FileError, FileResult},
};
use encoding_rs::WINDOWS_1252;

pub struct FDBRowHeaderAddrIterator<'a, T: ?Sized> {
    next_addr: u32,
    file: &'a mut T,
}

pub trait DatabaseBufReader
where
    Self: Seek + BufRead,
{
    /// Read a string from the file
    fn get_string(&mut self, addr: u32) -> io::Result<String>;
}

impl<T> DatabaseBufReader for T
where
    T: Seek + BufRead,
{
    fn get_string(&mut self, addr: u32) -> io::Result<String> {
        let mut string: Vec<u8> = Vec::new();
        self.seek(SeekFrom::Start(addr.into()))?;
        self.read_until(0x00, &mut string)?;
        if string.ends_with(&[0x00]) {
            string.pop();
        }
        let (decoded, _, _) = WINDOWS_1252.decode(&string);
        Ok(decoded.into_owned())
    }
}
impl<T> DatabaseReader for T where T: Seek + Read + ?Sized {}

/// Parse a struct at the give offset into the buffer
fn parse_at<R: Seek + Read + ?Sized, T>(
    reader: &mut R,
    addr: impl Into<u64>,
    buf: &mut [u8],
    parser: impl Fn(&[u8]) -> IResult<&[u8], T>,
) -> FileResult<T> {
    let addr = addr.into();
    reader.seek(SeekFrom::Start(addr))?;
    reader.read_exact(buf)?;
    let (_rest, header) = parser(buf).finish().map_err(|e| FileError::Parse {
        addr,
        offset: buf.len() - e.input.len(),
        code: e.code,
    })?;
    Ok(header)
}

fn bytes<IO: ParseLE>() -> IO::Buf {
    IO::Buf::default()
}

fn parse_list_at<R: Seek + Read + ?Sized, T: ParseFDB>(
    reader: &mut R,
    addr: impl Into<u64>,
    count: u32,
) -> FileResult<Vec<T>> {
    let addr = addr.into();
    reader.seek(SeekFrom::Start(addr))?;
    let buf_len = <T::IO as ParseLE>::BYTE_COUNT;
    let mut buf = bytes::<T::IO>();
    let mut offset = 0;
    let mut list = Vec::with_capacity(count as usize);
    for _ in 0..count {
        reader.read_exact(buf.as_mut())?;
        let (_rest, t) =
            parser::parse::<T>(buf.as_mut())
                .finish()
                .map_err(|e| FileError::Parse {
                    addr,
                    offset: offset + buf_len - e.input.len(),
                    code: e.code,
                })?;
        list.push(t);
        offset += buf_len;
    }
    Ok(list)
}
pub trait DatabaseReader
where
    Self: Seek + Read,
{
    /// Read the schema header
    fn get_header(&mut self) -> FileResult<FDBHeader> {
        let mut bytes = [0; 8];
        parse_at(self, 0u64, &mut bytes, parser::parse::<FDBHeader>)
    }

    /// Read the table header
    fn get_table_header_list(&mut self, header: FDBHeader) -> FileResult<FDBTableHeaderList> {
        let addr = header.table_header_list_addr;
        let count = header.table_count;
        parse_list_at(self, addr, count).map(FDBTableHeaderList::from)
    }

    /// Read the table def header
    fn get_table_def_header(&mut self, addr: u32) -> FileResult<FDBTableDefHeader> {
        let mut bytes = [0; FDBTableDefHeader::BYTE_COUNT];
        parse_at(self, addr, &mut bytes, parser::parse::<FDBTableDefHeader>)
    }

    /// Get a 64bit integer
    fn get_i64(&mut self, addr: u32) -> io::Result<i64> {
        let mut bytes: [u8; 8] = [0; 8];
        self.seek(SeekFrom::Start(addr.into()))?;
        self.read_exact(&mut bytes)?;
        Ok(i64::from_le_bytes(bytes))
    }

    /// Get the column header list
    fn get_column_header_list<'b>(
        &'b mut self,
        header: &FDBTableDefHeader,
    ) -> FileResult<FDBColumnHeaderList> {
        parse_list_at(self, header.column_header_list_addr, header.column_count)
            .map(FDBColumnHeaderList::from)
    }

    /// Get the table data header
    fn get_table_data_header(&mut self, addr: u32) -> FileResult<FDBTableDataHeader> {
        let mut bytes = bytes::<<FDBTableDataHeader as ParseFDB>::IO>();
        parse_at(self, addr, &mut bytes, parser::parse::<FDBTableDataHeader>)
    }

    /// Get the table bucket header list
    fn get_bucket_header_list(
        &mut self,
        header: &FDBTableDataHeader,
    ) -> FileResult<FDBBucketHeaderList> {
        let addr = header.bucket_header_list_addr;
        let count = header.bucket_count;
        parse_list_at(self, addr, count).map(FDBBucketHeaderList::from)
    }

    /// Get a row header list entry
    fn get_row_header_list_entry(&mut self, addr: u32) -> FileResult<FDBRowHeaderListEntry> {
        let mut bytes = [0; FDBRowHeaderListEntry::BYTE_COUNT];
        parse_at(
            self,
            addr,
            &mut bytes,
            parser::parse::<FDBRowHeaderListEntry>,
        )
    }

    /// Get a row header
    fn get_row_header(&mut self, addr: u32) -> FileResult<FDBRowHeader> {
        let mut bytes: [u8; 8] = [0; FDBRowHeader::BYTE_COUNT];
        parse_at(self, addr, &mut bytes, parser::parse::<FDBRowHeader>)
    }

    fn get_field_data_list(&mut self, header: FDBRowHeader) -> FileResult<FDBFieldDataList> {
        parse_list_at(self, header.field_data_list_addr, header.field_count)
            .map(FDBFieldDataList::from)
    }

    fn get_row_header_addr_iterator<'a>(
        &'a mut self,
        addr: u32,
    ) -> FDBRowHeaderAddrIterator<'a, Self> {
        FDBRowHeaderAddrIterator::<'a> {
            file: self,
            next_addr: addr,
        }
    }
}

impl<'a, T> Iterator for FDBRowHeaderAddrIterator<'a, T>
where
    T: Read + Seek,
{
    type Item = FileResult<u32>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_addr {
            std::u32::MAX => None,
            addr => match self.file.get_row_header_list_entry(addr) {
                Ok(entry) => {
                    self.next_addr = entry.row_header_list_next_addr;
                    Some(Ok(entry.row_header_addr))
                }
                Err(e) => {
                    self.next_addr = std::u32::MAX;
                    Some(Err(e))
                }
            },
        }
    }
}
