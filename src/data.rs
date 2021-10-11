use crate::{config::Config, label::LABELS};
use if_chain::if_chain;

#[derive(Debug, Clone)]
pub enum DataVal {
    DB(u8),
    DW(u16),
    DL(u32)
}
impl DataVal {
    pub fn as_u64(&self) -> u64 {
        match self {
            DataVal::DB(b) => *b as u64,
            DataVal::DW(w) => *w as u64,
            DataVal::DL(l) => *l as u64
        }
    }
}

#[derive(Debug, Clone)]
pub struct Data {
    pub address: u64,
    pub data: Vec<DataVal>,
    pub comment: Option<String>
}

impl Data {
    pub fn to_string(&self, config: &Config) -> String {
        let mut last_data_cmd = "";
        let mut output = "    ".to_string();
        let mut first_cmd = true;
        let mut first_val = true;
        let mut cur_pc = self.address;

        for d in &self.data {
            let (data_cmd, data_len) = match d {
                DataVal::DB(_) => ("db", 1),
                DataVal::DW(_) => ("dw", 2),
                DataVal::DL(_) => ("dl", 3)
            };

            let labels = LABELS.lock().unwrap();

            if !first_cmd && labels.contains_key(&cur_pc) {
                /* There's a label for this address, add it into the data */
                output.push_str(&format!(" : {}: ", labels[&cur_pc].name));
                first_cmd = true;
                first_val = true;
                last_data_cmd = "";
            }
            
            if data_cmd != last_data_cmd {
                output.push_str(&format!("{}{} ", if first_cmd { "" } else { " : " }, data_cmd));
                last_data_cmd = data_cmd;
                first_val = true;
                first_cmd = false;
            }

            if_chain! {
                if let Some(ov) = config.get_override(cur_pc);
                if let Some(t) = &ov._type;
                if t == "Pointer";
                then {
                    let db = ov.db.unwrap_or(cur_pc >> 16);
                    let label_addr = (d.as_u64() & 0xFFFF_u64) | (db << 16);
                    if labels.contains_key(&label_addr) {
                        output.push_str(&format!("{}{}", if first_val { "" } else { "," }, labels[&label_addr].name));
                    } else {
                        match d {                
                            DataVal::DB(db) => output.push_str(&format!("{}${:02X}", if first_val { "" } else { "," }, db)),
                            DataVal::DW(dw) => output.push_str(&format!("{}${:04X}", if first_val { "" } else { "," }, dw)),
                            DataVal::DL(dl) => output.push_str(&format!("{}${:06X}", if first_val { "" } else { "," }, dl)),
                        }       
                    }                                 
                } else {
                    match d {                
                        DataVal::DB(db) => output.push_str(&format!("{}${:02X}", if first_val { "" } else { "," }, db)),
                        DataVal::DW(dw) => output.push_str(&format!("{}${:04X}", if first_val { "" } else { "," }, dw)),
                        DataVal::DL(dl) => output.push_str(&format!("{}${:06X}", if first_val { "" } else { "," }, dl)),
                    }
                }
            }

            first_val = false;
            cur_pc += data_len;
        }

        if let Some(comment) = &self.comment {
            output.push_str(&format!(" ; | {:06X} | {}", self.address, comment));
        }

        output
    }    
}