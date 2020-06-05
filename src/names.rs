use cached::proc_macro::cached;
use crc::{crc32, Hasher32};
use indexmap::IndexMap;
use lazy_static::lazy_static;

const NAMES: &'static str = include_str!("../data/botw_hashed_names.txt");
const NUMBERED_NAMES: &'static str = include_str!("../data/botw_numbered_names.txt");

lazy_static! {
    pub static ref DEFAULT_TABLE: IndexMap<u32, String> = {
        let mut m: IndexMap<u32, String> = IndexMap::new();
        let mut dig = crc32::Digest::new(crc::crc32::IEEE);
        for name in NAMES.split("\n") {
            dig.write(name.as_bytes());
            m.insert(dig.sum32(), name.to_string());
            dig.reset();
        }
        m
    };
    static ref NUMBERED_NAME_LIST: Vec<String> =
        NUMBERED_NAMES.split("\n").map(|s| s.to_string()).collect();
}

#[cached]
pub fn get_default_name_table() -> NameTable {
    let def_table: IndexMap<u32, String> = DEFAULT_TABLE.clone();
    NameTable { table: def_table }
}

#[derive(Clone)]
pub struct NameTable {
    table: IndexMap<u32, String>,
}

impl NameTable {
    pub fn new(include_stock_names: bool) -> NameTable {
        let mut m: IndexMap<u32, String> = IndexMap::new();
        if include_stock_names {
            let mut dig = crc32::Digest::new(crc::crc32::IEEE);
            for name in NAMES.split("\n") {
                dig.write(name.as_bytes());
                m.insert(dig.sum32(), name.to_string());
                dig.reset();
            }
        }
        NameTable { table: m }
    }

    pub fn add_name(self: &mut NameTable, name: &str) {
        let mut digest = crc32::Digest::new(crc32::IEEE);
        digest.write(name.as_bytes());
        self.table.insert(digest.sum32(), name.to_owned());
        digest.reset();
    }

    pub fn get_name(self, crc: u32) -> Option<String> {
        match self.table.get(&crc) {
            Some(s) => Some(s.to_string()),
            None => None,
        }
    }
}

#[cached]
pub fn guess_name(crc: u32, parent_crc: u32, idx: usize) -> Option<String> {
    let parent = get_default_name_table().get_name(parent_crc);
    match parent {
        Some(parent_name) => {
            let mut digest = crc32::Digest::new(crc32::IEEE);
            fn possible_names(parent: &str, idx: usize) -> [String; 6] {
                [
                    format!("{}{}", parent, idx),
                    format!("{}_{}", parent, idx),
                    format!("{}{:02}", parent, idx),
                    format!("{}_{:02}", parent, idx),
                    format!("{}{:03}", parent, idx),
                    format!("{}_{:03}", parent, idx),
                ]
            }
            for i in idx..idx + 2 {
                for name in &possible_names(&parent_name, i) {
                    digest.write(name.as_bytes());
                    if digest.sum32() == crc {
                        return Some(name.to_string());
                    }
                    digest.reset();
                }
            }
            if &parent_name == "Children" {
                for i in idx..idx + 2 {
                    for name in &possible_names("Child", i) {
                        digest.write(name.as_bytes());
                        if digest.sum32() == crc {
                            return Some(name.to_string());
                        }
                        digest.reset();
                    }
                }
            }
            for suffix in &["s", "es", "List"] {
                if parent_name.ends_with(suffix) {
                    for i in idx..idx + 2 {
                        let mut alt_name = parent_name.clone();
                        alt_name.truncate(parent_name.len() - suffix.len());
                        for name in &possible_names(&alt_name, i) {
                            digest.write(name.as_bytes());
                            if digest.sum32() == crc {
                                return Some(name.to_string());
                            }
                            digest.reset();
                        }
                    }
                }
            }
            try_numbered_name(idx, crc)
        }
        None => try_numbered_name(idx, crc),
    }
}

#[cached]
fn try_numbered_name(idx: usize, crc: u32) -> Option<String> {
    let mut opt = Option::None;
    let mut dig = crc32::Digest::new(crc32::IEEE);
    for name in NUMBERED_NAME_LIST.iter() {
        for i in 0..idx + 2 {
            let mut maybe: String = name.to_string();
            if name.contains("{}") {
                maybe = maybe.replace("{}", &format!("{}", i));
            }
            if name.contains("{:02}") {
                maybe = maybe.replace("{:02}", &format!("{:02}", i));
            }
            if name.contains("{:03}") {
                maybe = maybe.replace("{:03}", &format!("{:03}", i));
            }
            if name.contains("{:04}") {
                maybe = maybe.replace("{:04}", &format!("{:03}", i));
            }
            dig.write(maybe.as_bytes());
            if dig.sum32() == crc as u32 {
                opt = Some(maybe);
            }
            dig.reset();
        }
        dig.reset();
    }
    opt
}
