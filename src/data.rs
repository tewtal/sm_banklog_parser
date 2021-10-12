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

            let mut labels = LABELS.lock().unwrap();

            if !first_cmd && labels.contains_key(&cur_pc) {
                /* There's a label for this address, add it into the data */
                output.push_str(&format!(" : {}: ", labels[&cur_pc].name));
                let mut lbl = labels.get_mut(&cur_pc).unwrap();
                lbl.assigned = true;
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
                if t == "Pointer" || t == "Data";
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
                    if_chain! {
                        if let Some(ov) = config.get_override(cur_pc);
                        if let Some(t) = &ov._type;
                        if t == "Struct";
                        if let Some(st) = config.structs.iter().find(|s| &s.name == ov._struct.as_ref().unwrap_or(&"".to_string()));
                        then {
                            let last_field = &st.fields[st.fields.len() - 1];
                            let st_len = last_field.offset + last_field.length;
                            let cur_offset = cur_pc - self.address;
                            let cur_st_offset = cur_offset % st_len;
                            let field = &st.fields.iter().find(|f| f.offset == cur_st_offset).unwrap();
                            let db = field.db.unwrap_or(cur_pc >> 16);                                    
                            let label_addr = if field.length < 3 { (d.as_u64() & 0xFFFF_u64) | (db << 16) } else { d.as_u64() };
                            if field._type == "Pointer" && (label_addr & 0xFFFF_u64) >= 0x8000 && labels.contains_key(&label_addr) {
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