use crate::{label::{self, LabelType}, opcode::{Opcode, AddrMode}};

#[derive(Debug, Clone)]
pub enum ArgType {
    None,
    Address(u64),
    BlockMove(u8, u8)
}

#[derive(Debug, Clone)]
pub struct Code {
    pub address: u64,
    pub opcode: &'static Opcode,
    pub arg: ArgType,
    pub comment: Option<String>,
    pub length: u8,
    pub db: u8,
}

impl Code {
    fn arg_label(&self) -> String {
        /* TODO: Get label if exists */
        /* Make sure to handle PC-relative addresses correctly */
        match self.arg {
            ArgType::Address(addr) => {
                let label_addr = match self.length {
                    1 => 0x7E0000 | (addr & 0xFF),
                    2 => match addr {
                        0..=0x1FFF => 0x7E0000 | (addr & 0xFFFF),
                        0x2000..=0x7FFF => (addr & 0xFFFF),
                        _ => ((self.db as u64) << 16) | (addr & 0xFFFF)
                    },
                    3 => addr,
                    _ => panic!("Invalid argument length")
                };

                let labels = label::LABELS.lock().unwrap();

                let label = {
                    if label_addr > 0x808000 {
                        if labels.contains_key(&label_addr) {
                            (Some(&labels[&label_addr]), 0)
                        } else if labels.contains_key(&(label_addr - 1)) {
                            (Some(&labels[&(label_addr - 1)]), -1)
                        } else if labels.contains_key(&(label_addr + 1)) {
                            (Some(&labels[&(label_addr + 1)]), 1)
                        } else if labels.contains_key(&(label_addr - 2)) {
                            (Some(&labels[&(label_addr - 2)]), -2)
                        } else if labels.contains_key(&(label_addr + 2)) {
                            (Some(&labels[&(label_addr + 2)]), 2)
                        } else {
                            (None, 0)    
                        }
                    } else {
                        (None, 0)
                    }
                };

                match label {
                    (Some(l), offset) => {
                        if ((l.name.starts_with("IMM") && self.opcode.addr_mode == AddrMode::Immediate) || self.opcode.addr_mode != AddrMode::Immediate) && l.label_type != LabelType::Blocked {
                            match offset {
                                0 => l.name.to_string(),
                                -1 | -2 => format!("{}+{}", l.name, -offset),
                                1 | 2 => format!("{}{}", l.name, -offset),
                                _ => panic!("Invalid argument length")
                            }
                        } else {
                            match self.length {
                                1 => format!("${:02X}", addr),
                                2 => format!("${:04X}", addr),
                                3 => format!("${:06X}", addr),
                                _ => panic!("Invalid argument length")
                            }                                
                        }
                    },
                    (None, _) => {
                        match self.length {
                            1 => format!("${:02X}", addr),
                            2 => format!("${:04X}", addr),
                            3 => format!("${:06X}", addr),
                            _ => panic!("Invalid argument length")
                        }                        
                    }
                }
            },
            ArgType::BlockMove(src, dst) => {
                format!("${:02X},${:02X}", src, dst)
            },
            _ => panic!("Tried to format a None-argument")
        }
    }
}

impl ToString for Code {
    fn to_string(&self) -> String {
        let opcode = match self.opcode.addr_mode {
            AddrMode::Absolute =>                       format!("{}.w {}", self.opcode.name, self.arg_label()),
            AddrMode::AbsoluteIndexedIndirect =>        format!("{} ({},X)", self.opcode.name, self.arg_label()),
            AddrMode::AbsoluteIndexedLong =>            format!("{}.l {},X", self.opcode.name, self.arg_label()),
            AddrMode::AbsoluteIndexedX =>               format!("{}.w {},X", self.opcode.name, self.arg_label()),
            AddrMode::AbsoluteIndexedY =>               format!("{}.w {},Y", self.opcode.name, self.arg_label()),
            AddrMode::AbsoluteIndirect =>               format!("{} ({})", self.opcode.name, self.arg_label()),
            AddrMode::AbsoluteIndirectLong =>           format!("{} [{}]", self.opcode.name, self.arg_label()),
            AddrMode::AbsoluteLong =>                   format!("{}.l {}", self.opcode.name, self.arg_label()),
            AddrMode::BlockMove =>                      format!("{} {}", self.opcode.name, self.arg_label()),
            AddrMode::Direct =>                         format!("{}.b {}", self.opcode.name, self.arg_label()),
            AddrMode::DirectIndexedIndirect =>          format!("{}.b ({},X)", self.opcode.name, self.arg_label()),
            AddrMode::DirectIndexedX =>                 format!("{}.b {},X", self.opcode.name, self.arg_label()),
            AddrMode::DirectIndexedY =>                 format!("{}.b {},Y", self.opcode.name, self.arg_label()),
            AddrMode::DirectIndirect =>                 format!("{}.b ({})", self.opcode.name, self.arg_label()),
            AddrMode::DirectIndirectIndexed =>          format!("{}.b ({}),Y", self.opcode.name, self.arg_label()),
            AddrMode::DirectIndirectIndexedLong =>      format!("{}.b [{}],Y", self.opcode.name, self.arg_label()),
            AddrMode::DirectIndirectLong =>             format!("{}.b [{}]", self.opcode.name, self.arg_label()),
            AddrMode::Immediate =>                      format!("{}.{} #{}", self.opcode.name, if self.length == 1 { "b" } else { "w" }, self.arg_label()),
            AddrMode::ImmediateByte =>                  format!("{}.b #{}", self.opcode.name, self.arg_label()),
            AddrMode::Implied =>                        self.opcode.name.to_string(),
            AddrMode::Relative =>                       format!("{} {}", self.opcode.name, self.arg_label()),
            AddrMode::RelativeLong =>                   format!("{} {}", self.opcode.name, self.arg_label()),
            AddrMode::StackRelative =>                  format!("{} {},S", self.opcode.name, self.arg_label()),
            AddrMode::StackRelativeIndirectIndexed =>   format!("{} ({},S),Y", self.opcode.name, self.arg_label()),
        };

        format!("    {:<40};| {:06X} | {:02X} | {}", opcode, self.address, self.db, self.comment.as_ref().unwrap_or(&"".to_owned()))
    }
}