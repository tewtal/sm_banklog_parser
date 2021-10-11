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
                            let label_addr = ((c.db as u64) << 16) | (arg_addr & 0xFFFF);
                            Some(Label {
                                address: label_addr,
                                name: format!("PTR_{:06X}", label_addr),
                                label_type: LabelType::PointerTable(0),
                                assigned: false
                            })                            
                        },
                        Opcode { addr_mode: AddrMode::AbsoluteIndexedLong, .. } |
                        Opcode { addr_mode: AddrMode::AbsoluteIndexedX, .. } |
                        Opcode { addr_mode: AddrMode::AbsoluteIndexedY, .. } if (arg_addr & 0xFFFF) >= 0x8000 => {
                             /* Most likely a table of data somewhere being indexed (since it's in rom) */
                             let label_addr = if c.opcode.addr_mode != AddrMode::AbsoluteIndexedLong { ((c.db as u64) << 16) | (arg_addr & 0xFFFF) } else { arg_addr };
                             Some(Label {
                                 address: label_addr,
                                 name: format!("TBL_{:06X}", label_addr),
                                 label_type: LabelType::DataTable(0),
                                 assigned: false
                             })                              
                        }
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
                        cur_pc += data_len;
                    }
                    None                    
                }
                _ => None            
            };

            if let Some(label) = label {
                if !labels.contains_key(&(label.address - 1)) && !labels.contains_key(&(label.address + 1)) && 
                   !labels.contains_key(&(label.address - 2)) && !labels.contains_key(&(label.address + 2)) {
                       labels.entry(label.address).or_insert(label);
                }
            }
        }
    }
}
