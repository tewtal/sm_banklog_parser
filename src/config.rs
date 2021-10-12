use serde::{Deserialize};
use glob::glob;

#[derive(Debug, PartialEq, Deserialize)]
pub struct StructField {
    pub name: String,
    pub offset: u64,
    pub length: u64,
    #[serde(rename = "type")]
    pub _type: String,
    pub db: Option<u64>
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<StructField>
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct Label {
    pub addr: u64,
    pub name: String,
    #[serde(rename = "type")]
    pub label_type: Option<String>,
    pub length: Option<u64>
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum OverrideAddr {
    Address(u64),
    Range(Vec<u64>)
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct Override {
    pub addr: OverrideAddr,
    pub db: Option<u64>,
    #[serde(rename ="type")]
    pub _type: Option<String>,
    #[serde(rename ="struct")]
    pub _struct: Option<String>,
    pub opcode: Option<Vec<u64>>,
}

#[derive(Debug, PartialEq)]
pub struct Config {
    pub labels: Vec<Label>,
    pub overrides: Vec<Override>,
    pub structs: Vec<Struct>
}

impl Config {
    pub fn load(path: &str) -> Config {
        let label_filenames = glob(&format!("{}/labels/*.yaml", path)).unwrap();        
        let labels: Vec<Label> = label_filenames.flatten()
            .map(|f| serde_yaml::from_str::<Vec<Label>>(&std::fs::read_to_string(f).unwrap()).unwrap())
            .flatten().collect();

            let override_filenames = glob(&format!("{}/overrides/*.yaml", path)).unwrap();        
            let overrides: Vec<Override> = override_filenames.flatten()
                .map(|f| serde_yaml::from_str::<Vec<Override>>(&std::fs::read_to_string(f).unwrap()).unwrap())
                .flatten().collect();

            let struct_filenames = glob(&format!("{}/structs/*.yaml", path)).unwrap();        
            let structs: Vec<Struct> = struct_filenames.flatten()
                .map(|f| serde_yaml::from_str::<Vec<Struct>>(&std::fs::read_to_string(f).unwrap()).unwrap())
                .flatten().collect();

        Config { labels, overrides, structs }
    }
    
    pub fn get_override(&self, addr: u64) -> Option<&Override> {
        self.overrides.iter().find(|o| match &o.addr {
            OverrideAddr::Address(a) if *a == addr => true,
            OverrideAddr::Range(r) if addr >= r[0] && addr <= r[1] => true,
            _ => false
        })
    }
}