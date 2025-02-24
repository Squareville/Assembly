#![allow(clippy::upper_case_acronyms)]
//! # Data definitions for sysdiagrams
use ms_oforms::properties::types::{Position, Size};
use std::collections::HashMap;

#[derive(Debug)]
pub struct SchGrid {
    pub d1: u32,
    pub d2: u32,
    pub d3: u32,
    pub d4: u32,
    //pub d5: [u32; 6],
    pub d6: u32,
    //pub d7: [u32;16],
    //pub d8: [u32;16],
    pub d9: u32,
    //pub d10: [u32;16],
    //pub d11: [u32;11],
    pub d12: u32,
    //pub d13: [u32;2],
    pub d14: Vec<u32>,
    pub size1: Size,
    pub size2: Size,
    pub name: String,
    pub table: String,
    pub schema: String,
}

#[derive(Debug)]
pub struct Control1 {
    pub positions: Vec<Position>,
    pub pos: Position,
    pub d1: u16,
    //pub d2: [u8; 32],
    pub d3: u32,
    pub d4: u32,
    pub d5: u32,
    pub d6: u32,
    pub d7: u32,
    //pub d8: [u8; 6],
    pub d9: u32,
}

#[derive(Debug, Clone)]
pub struct DSRefSchemaEntry {
    pub k1: u32,
    pub table: String,
    pub schema: String,
}

#[derive(Debug)]
pub struct DSRefSchemaContents {
    pub name: String,
    pub guid: String,
    pub tables: Vec<DSRefSchemaEntry>,
    pub settings: HashMap<String, String>,
}

#[derive(Debug)]
pub struct Table {
    pub sch_grid: SchGrid,
    pub caption: String,
}

#[derive(Debug)]
pub struct Relationship {
    pub control: Control1,
    pub caption: String,
    pub from: String,
    pub to: String,
    pub name: String,
}

#[derive(Debug)]
pub struct SysDiagram {
    pub tables: Vec<Table>,
    pub relationships: Vec<Relationship>,
    pub dsref_schema_contents: DSRefSchemaContents,
}
