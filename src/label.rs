use std::collections::{BTreeMap, HashMap};
use if_chain::if_chain;
use lazy_static::lazy_static;
use std::sync::Mutex;

use crate::{code::ArgType, config::Config, data::DataVal, line::Line, opcode::{AddrMode, Opcode}};

lazy_static! {
    pub static ref LABELS: Mutex<HashMap<u64, Label>> = Mutex::new(HashMap::new());
}


#[derive(Debug, PartialEq)]
pub enum LabelType {
    Undefined,
    Subroutine,
    Branch,
    Data,
    PointerTable(u64),
    DataTable(u64),
    Blocked
}

#[derive(Debug)]
pub struct Label {
    pub address: u64,
    pub name: String,
    pub label_type: LabelType,
    pub assigned: bool
}

pub fn generate_labels(lines: &BTreeMap<u64, Vec<Line>>, config: &Config) {
    let mut labels = LABELS.lock().unwrap();

    /* Pre-initialize all labels from the config file */
    for label in &config.labels {
        let length = label.length.unwrap_or(0);
        let label_type = match label.label_type.as_ref().unwrap_or(&"Data".to_string()).as_str() {
            "Subroutine" => LabelType::Subroutine,
            "Branch" => LabelType::Branch,
            "DataTable" => LabelType::DataTable(length),
            "PointerTable" => LabelType::PointerTable(length),
            "Data" => LabelType::Data,
            "Blocked" => LabelType::Blocked,
            _ => LabelType::Undefined
        };

        labels.insert(label.addr, Label { 
            address: label.addr, 
            name: label.name.clone(), 
            label_type, 
            assigned: false 
        });
    }

    for (addr, line) in lines {
        for addr_line in line {
            let label = match addr_line {
                Line::Code(c) => match c.arg {
                    ArgType::Address(arg_addr) => match c.opcode {
                        Opcode { name: "JSR", addr_mode: AddrMode::Absolute, .. } |
                        Opcode { name: "JSL", .. } => {
                            let label_addr = if c.opcode.name == "JSR" { (addr & 0xFF0000) | (arg_addr & 0xFFFF) } else { arg_addr };
                            Some(Label {
                                address: label_addr,
                                name: format!("SUB{}_{:06X}", if c.opcode.name == "JSL" { "L" } else { "" }, label_addr),
                                label_type: LabelType::Subroutine,
                                assigned: false
                            })
                        },
                        Opcode { addr_mode: AddrMode::AbsoluteIndexedIndirect, .. } => {
                            /* Anything using this is using a table of pointers (generally) */
                            let arg_addr = ((c.db as u64) << 16) | (arg_addr & 0xFFFF);
                            let bank = arg_addr >> 16;
                            let low_addr = arg_addr & 0xFFFF_u64;
                            let (label_addr, prefix) = match low_addr {
                                0x00..=0xFF if (bank < 0x70 || bank > 0x7F) || bank == 0x7E => (0, ""), // Don't label DP for now
                                0x100..=0x1FFF if (bank < 0x70 || bank > 0x7F) || bank == 0x7E => (0x7E0000 | (low_addr & 0xFFFF), "LORAM_PTR"),
                                0x2000..=0x7FFF if bank < 0x40 || bank >= 0x80 => ((low_addr & 0xFFFF), "HW_PTR"),
                                _ if bank == 0x7E || bank == 0x7F => (arg_addr, "WRAM_PTR"),
                                _ if bank >= 0x70 && bank < 0x7E => (arg_addr, "SRAM_PTR"),
                                _ => (arg_addr, "PTR")
                            };
                            if label_addr > 0 {
                                Some(Label {
                                    address: label_addr,
                                    name: format!("{}_{:06X}", prefix, label_addr),
                                    label_type: LabelType::PointerTable(0),
                                    assigned: false
                                })                 
                            } else {
                                None
                            }                                
                        },
                        Opcode { addr_mode: AddrMode::AbsoluteIndexedLong, .. } |
                        Opcode { addr_mode: AddrMode::AbsoluteIndexedX, .. } |
                        Opcode { addr_mode: AddrMode::AbsoluteIndexedY, .. } if (arg_addr & 0xFFFF) >= 0x0100 => {
                            let arg_addr = if c.opcode.addr_mode != AddrMode::AbsoluteLong { ((c.db as u64) << 16) | (arg_addr & 0xFFFF) } else { arg_addr };
                            let bank = arg_addr >> 16;
                            let low_addr = arg_addr & 0xFFFF_u64;
                            let (label_addr, prefix) = match low_addr {
                                0x00..=0xFF if (bank < 0x70 || bank > 0x7F) || bank == 0x7E => (0, ""), // Don't label DP for now
                                0x100..=0x1FFF if (bank < 0x70 || bank > 0x7F) || bank == 0x7E => (0x7E0000 | (low_addr & 0xFFFF), "LORAM_TBL"),
                                0x2000..=0x7FFF if bank < 0x40 || bank >= 0x80 => ((low_addr & 0xFFFF), "HW_TBL"),
                                _ if bank == 0x7E || bank == 0x7F => (arg_addr, "WRAM_TBL"),
                                _ if bank >= 0x70 && bank < 0x7E => (arg_addr, "SRAM_TBL"),
                                _ => (arg_addr, "TBL")
                            };
                            if label_addr > 0 {
                                Some(Label {
                                    address: label_addr,
                                    name: format!("{}_{:06X}", prefix, label_addr),
                                    label_type: LabelType::DataTable(0),
                                    assigned: false
                                })                 
                            } else {
                                None
                            }             
                        },
                        Opcode { addr_mode: AddrMode::Immediate, .. } => {
                            /* For now, only do this with overrides */
                            if let Some(ov) = config.get_override(*addr) {
                                if ov._type.as_ref().unwrap_or(&"".to_string()) == "Pointer" {
                                    let db = ov.db.unwrap_or(addr >> 16);
                                    let label_addr = (arg_addr & 0xFFFF_u64) | (db << 16);
                                    Some(Label {
                                        address: label_addr,
                                        name: format!("IMM_{:06X}", label_addr),
                                        label_type: LabelType::DataTable(0),
                                        assigned: false
                                    })
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        },
                        Opcode { addr_mode: AddrMode::Relative, .. } => {
                            /* Branches */
                            let label_addr = ((*addr as i64) + 2 + (((arg_addr & 0xFF) as i8)) as i64) as u64;
                            Some(Label {
                                address: label_addr,
                                name: format!("BRA_{:06X}", label_addr),
                                label_type: LabelType::Branch,
                                assigned: false
                            })                            
                        },
                        Opcode { addr_mode: AddrMode::Absolute, .. } |
                        Opcode { addr_mode: AddrMode::AbsoluteLong, .. } => {
                            let arg_addr = if c.opcode.addr_mode != AddrMode::AbsoluteLong { ((c.db as u64) << 16) | (arg_addr & 0xFFFF) } else { arg_addr };
                            let bank = arg_addr >> 16;
                            let low_addr = arg_addr & 0xFFFF_u64;
                            let (label_addr, prefix) = match low_addr {
                                0x00..=0xFF if (bank < 0x70 || bank > 0x7F) || bank == 0x7E => (0, ""), // Don't label DP for now
                                0x100..=0x1FFF if (bank < 0x70 || bank > 0x7F) || bank == 0x7E => (0x7E0000 | (low_addr & 0xFFFF), "LORAM"),
                                0x2000..=0x7FFF if bank < 0x40 || bank >= 0x80 => ((low_addr & 0xFFFF), "HWREG"),
                                _ if bank == 0x7E || bank == 0x7F => (arg_addr, "WRAM"),
                                _ if bank >= 0x70 && bank < 0x7E => (arg_addr, "SRAM"),
                                _ if c.opcode.name == "PEA" => (arg_addr + 1, "SUB"),
                                _ => (arg_addr, "DAT")
                            };

                            if label_addr > 0 {
                                Some(Label {
                                    address: label_addr,
                                    name: format!("{}_{:06X}", prefix, label_addr),
                                    label_type: if prefix != "SUB" { LabelType::Data } else { LabelType::Subroutine }, 
                                    assigned: false
                                })
                            } else {
                                None
                            }
                        },
                        _ => None
                    },
                    _ => None
                },
                Line::Data(data) => {
                    /* Scan through data and insert labels for data pointers (from overrides) */
                    let mut cur_pc = data.address;
                    for d in &data.data {
                        let data_len = match d {
                            DataVal::DB(_) => 1,
                            DataVal::DW(_) => 2,
                            DataVal::DL(_) => 3
                        };
                              
                        /* Handle regular pointer overrides */
                        if_chain! {
                            if let Some(ov) = config.get_override(cur_pc);
                            if let Some(t) = &ov._type;
                            if t == "Pointer";
                            then {
                                let db = ov.db.unwrap_or(cur_pc >> 16);
                                let label_addr = (d.as_u64() & 0xFFFF_u64) | (db << 16);
                                labels.entry(label_addr).or_insert(Label { 
                                    address: label_addr, 
                                    name: format!("SUB_{:06X}", label_addr), label_type: LabelType::Subroutine, assigned: false });
                            }
                        }

                        /* Handle struct overrides */
                        if_chain! {
                            if let Some(ov) = config.get_override(cur_pc);
                            if let Some(t) = &ov._type;
                            if t == "Struct";
                            then {
                                if let Some(st) = config.structs.iter().find(|s| &s.name == ov._struct.as_ref().unwrap_or(&"".to_string())) {                                    
                                    let last_field = &st.fields[st.fields.len() - 1];
                                    let st_len = last_field.offset + last_field.length;
                                    let cur_offset = cur_pc - data.address;
                                    let cur_st_offset = cur_offset % st_len;
                                    let field = &st.fields.iter().find(|f| f.offset == cur_st_offset).unwrap();
                                    if field._type == "Pointer" {
                                        let db = field.db.unwrap_or(cur_pc >> 16);                                    
                                        let label_addr = if field.length < 3 { (d.as_u64() & 0xFFFF_u64) | (db << 16) } else { d.as_u64() };
                                        if (label_addr & 0xFFFF) >= 0x8000 {
                                            labels.entry(label_addr).or_insert(Label { 
                                                address: label_addr, 
                                                name: format!("SUB_{:06X}", label_addr), label_type: LabelType::Subroutine, assigned: false });
                                        }
                                    }
                                }
                            }
                        }

                        cur_pc += data_len;
                    }
                    None                    
                }
                _ => None            
            };

            if let Some(label) = label {
                if let LabelType::DataTable(_) = label.label_type {
                    if !labels.contains_key(&(label.address - 1)) && !labels.contains_key(&(label.address + 1)) && 
                       !labels.contains_key(&(label.address - 2)) && !labels.contains_key(&(label.address + 2)) {
                            labels.entry(label.address).or_insert(label);
                       }
                } else if let LabelType::PointerTable(_) = label.label_type {
                    if !labels.contains_key(&(label.address - 1)) && !labels.contains_key(&(label.address + 1)) && 
                       !labels.contains_key(&(label.address - 2)) && !labels.contains_key(&(label.address + 2)) {
                            labels.entry(label.address).or_insert(label);
                       }
                } else {
                    labels.entry(label.address).or_insert(label);
                }
            }
        }
    }
}
