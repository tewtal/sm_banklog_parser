use std::{collections::BTreeMap, fs::File, io::{BufRead, BufReader, Write}};
use regex::Regex;
use glob::glob;

mod code;
mod opcode;
mod data;
mod label;
mod line;
mod config;

use data::{Data};
use code::{Code, ArgType};
use label::LABELS;
use line::Line;

use crate::label::LabelType;


fn main() -> Result<(), Box<dyn std::error::Error>> {

    let filename_regex = Regex::new(r"Bank \$([0-9A-F]{2})(\.\.\$([0-9A-F]{2})|)").unwrap();
    let mut bank_groups: Vec<(u8, u8)> = Vec::new();
    let config = config::Config::load("./config/");

    let mut lines: BTreeMap<u64, Vec<Line>> = BTreeMap::new();
    let filenames = glob("./logs/*.asm").unwrap();
    for filename in filenames.flatten() {
        let cap = filename_regex.captures(filename.to_str().unwrap()).unwrap();
        
        let bank_group = 
            (u8::from_str_radix(&cap[1], 16).unwrap(), 
                if let Some(c) = cap.get(3) {
                    if !c.as_str().trim().is_empty() {
                        u8::from_str_radix(c.as_str(), 16).unwrap() 
                    } else {
                        u8::from_str_radix(&cap[1], 16).unwrap()                             
                    }
                } else { 
                    u8::from_str_radix(&cap[1], 16).unwrap() 
                }
            );

        bank_groups.push(bank_group);

        let file = File::open(filename).unwrap();
        let reader = BufReader::new(file);
        let mut cur_addr = 0x008000 | ((bank_group.0 as u64) << 16);
        
        /* Parse the full file into data */
        for (addr, line) in reader.lines().flatten().map(|l| Line::parse(&l, &config)) {
            cur_addr = addr.unwrap_or(cur_addr);
            lines.entry(cur_addr).or_insert_with(Vec::new).push(line);
        }
    }
    
    /* copy enemy-banks to respective new bank */
    let enemy_banks = vec![0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xB2, 0xB3];
    let enemy_lines: BTreeMap<u64, Vec<Line>> = lines.iter().filter(|(k, _)| **k >= 0xA08000 && **k <= 0xA08686).map(|(k,v)| (*k, v.clone())).collect();
    for addr_line in &enemy_lines {
        for bank in &enemy_banks {
            let mut new_lines = Vec::new();
            let new_addr = bank << 16 | *addr_line.0 & (0xFFFF_u64);
            for line in addr_line.1 {
                let new_line = match line {
                    Line::Code(c) => {
                        let new_arg = match c.arg {
                            ArgType::Address(a) => {
                                if c.length == 3 && (new_addr & 0xFFFF) > 0x804D {
                                    ArgType::Address(bank << 16 | a & (0xFFFF_u64))
                                } else { ArgType::Address(a) }
                            },
                            ArgType::None => ArgType::None,
                            ArgType::BlockMove(a, b) => ArgType::BlockMove(a, b)
                        };

                        Line::Code(Code {
                            address: new_addr,
                            comment: c.comment.clone(),
                            length: c.length,
                            opcode: c.opcode,
                            db: *bank as u8,
                            arg: new_arg
                        })
                    },
                    Line::Data(d) => {
                        let new_data_addr = bank << 16 | d.address & (0xFFFF_u64);
                        Line::Data(Data {
                            address: new_data_addr,
                            comment: d.comment.clone(),
                            data: d.data.clone()
                        })
                    },
                    Line::Comment(c) => Line::Comment(c.to_string())
                };

                new_lines.push(new_line);
            }

            lines.insert(new_addr, new_lines);
        }
    }

    /* Autogenerate labels */
    label::generate_labels(&lines, &config);

    let mut output_file = File::create("./asm/main.asm").unwrap();
    let _ = writeln!(output_file, "lorom");
    let _ = writeln!(output_file, "incsrc labels.asm");
    for group in 0x80..0xE0 {
        let _ = writeln!(output_file, "incsrc bank_{:02X}.asm", group);
    }

    let mut cur_bank = 0;
    for (addr, line) in &lines {
        let bank = (addr >> 16) as u8;

        if bank != cur_bank {
            let _ = writeln!(output_file, "check bankcross on");

            let first_entry = lines.iter().find(|(k, v)| **k >= (((bank as u64) << 16) | 0x8000) && v.iter().any(|l| matches!(l, Line::Code(_) | Line::Data(_)))).unwrap();
            let first_address = if (first_entry.0 >> 16) == bank as u64 { first_entry.0 } else { addr };

            cur_bank = bank;
            output_file = File::create(format!("./asm/bank_{:02X}.asm", cur_bank)).unwrap();
            let _ = writeln!(output_file, "org ${:06X}\ncheck bankcross off", first_address);
        }
        
        {
            /* Make a temporary scope here so that the labels mutex lock falls out of scope before the rest of the code,
               otherwise it will deadlock. */
            let mut labels = LABELS.lock().unwrap();
            if labels.contains_key(addr) {
                let _ = writeln!(output_file, "{}{}", labels[addr].name, if labels[addr].name.starts_with(".") { "" } else { ":" });
                let mut label = labels.get_mut(addr).unwrap();
                label.assigned = true;
            }
        }

        for addr_line in line {
            let _ = writeln!(output_file, "{}", addr_line.to_string(&config));
        }

    }

    let labels = LABELS.lock().unwrap();
    output_file = File::create("./asm/labels.asm").unwrap();
    for (a, l) in labels.iter().filter(|(_,l)| !l.assigned && l.label_type != LabelType::Blocked) {
        let _ = writeln!(output_file, "{} = ${:06X}", l.name, a);
    }

    Ok(())
}
