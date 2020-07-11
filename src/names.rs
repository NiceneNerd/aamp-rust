use cached::proc_macro::cached;
use crc::{crc32, Hasher32};
use lazy_static::lazy_static;
use metrohash::MetroHashMap;
use std::sync::Mutex;

const NAMES: &str = include_str!("../data/botw_hashed_names.txt");
const NUMBERED_NAMES: &str = include_str!("../data/botw_numbered_names.txt");

lazy_static! {
    static ref NUMBERED_NAME_LIST: Vec<String> =
        NUMBERED_NAMES.split('\n').map(|s| s.to_string()).collect();
}

#[cached]
pub fn get_default_name_table() -> NameTable {
    NameTable::new(true)
}

lazy_static::lazy_static! {
    pub(crate) static ref TABLE: Mutex<NameTable> = Mutex::new(get_default_name_table());
}

#[derive(Clone)]
pub struct NameTable {
    table: MetroHashMap<u32, String>,
}

impl NameTable {
    pub fn new(include_stock_names: bool) -> NameTable {
        let mut m: MetroHashMap<u32, String> = MetroHashMap::default();
        if include_stock_names {
            let mut dig = crc32::Digest::new(crc::crc32::IEEE);
            for name in NAMES.split('\n') {
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

    pub fn get_name(&self, crc: u32) -> Option<String> {
        match self.table.get(&crc) {
            Some(s) => Some(s.to_string()),
            None => None,
        }
    }
}

lazy_static::lazy_static! {
    static ref DIGEST: Mutex<crc32::Digest> = Mutex::new(crc32::Digest::new(crc32::IEEE));
}

fn test_names(parent: &str, idx: usize, crc: u32) -> Option<String> {
    let mut digest = DIGEST.lock().unwrap();
    for i in &[idx, idx + 1] {
        for name in &[
            format!("{}{}", parent, i),
            format!("{}_{}", parent, i),
            format!("{}{:02}", parent, i),
            format!("{}_{:02}", parent, i),
            format!("{}{:03}", parent, i),
            format!("{}_{:03}", parent, i),
        ] {
            digest.write(name.as_bytes());
            if digest.sum32() == crc {
                return Some(name.to_string());
            }
            digest.reset();
        }
    }
    None
}

#[cached]
pub fn guess_name(crc: u32, parent_crc: u32, idx: usize) -> Option<String> {
    let table = TABLE.lock().unwrap();
    let parent = table.get_name(parent_crc);
    drop(table);
    match parent {
        Some(parent_name) => {
            let mut matched = test_names(&parent_name, idx, crc);
            if matched.is_none() {
                if &parent_name == "Children" {
                    matched = test_names("Child", idx, crc);
                }
                if matched.is_none() {
                    for suffix in &["s", "es", "List"] {
                        if parent_name.ends_with(suffix) {
                            matched = test_names(
                                &parent_name[0..parent_name.len() - suffix.len()],
                                idx,
                                crc,
                            );
                            if matched.is_some() {
                                break;
                            }
                        }
                    }
                }
            }
            match matched {
                Some(s) => Some(s),
                None => try_numbered_name(idx, crc),
            }
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
            let maybe: String = if name.contains('{') {
                rt_format(name, i)
            } else {
                name.to_string()
            };
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

fn rt_format(name: &str, i: usize) -> String {
    if name.contains("{}") {
        name.replace("{}", &format!("{}", i))
    } else if name.contains("{:02}") {
        name.replace("{:02}", &format!("{:02}", i))
    } else if name.contains("{:03}") {
        name.replace("{:03}", &format!("{:03}", i))
    } else if name.contains("{:04}") {
        name.replace("{:04}", &format!("{:04}", i))
    } else {
        unreachable!()
    }
}
