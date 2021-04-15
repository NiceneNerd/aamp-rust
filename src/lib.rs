#![feature(const_fn, seek_stream_len)]
//! # Nintendo parameter archive (AAMP) library in Rust
//!
//! A simple to use library for reading, writing, and converting Nintendo parameter archive (AAMP) files
//! in Rust. Supports only AAMP version 2, used in _The Legend of Zelda: Breath of the Wild_. Can
//! convert from AAMP to readable, editable YAML and back.
//!
//! ```rust
//! use aamp::ParameterIO;
//! let mut file = std::fs::File::open("test/Enemy_Lizalfos_Electric.bchemical").unwrap();
//! // Read an AAMP ParameterIO from any reader that implements Seek + Read
//! let pio = ParameterIO::from_binary(&mut file).unwrap();
//! for list in pio.lists.iter() {
//!     // Do stuff with lists
//! }
//! for obj in pio.objects.iter() {
//!     // Do stuff with objects
//! }
//! // Dumps YAML representation to a String
//! let yaml_dump: String = pio.to_text().unwrap();
//! ```
use crc::{crc32, Hasher32};
use indexmap::IndexMap;
pub mod names;
mod parse;
pub mod types;
mod write;
mod yaml;

#[derive(Debug, PartialEq, Clone)]
/// Represents a single AAMP parameter
pub enum Parameter {
    Bool(bool),
    F32(f32),
    Int(i32),
    Vec2(types::Vec2),
    Vec3(types::Vec3),
    Vec4(types::Vec4),
    Color(types::Color),
    String32(String),
    String64(String),
    Curve1(types::Curve1),
    Curve2(types::Curve2),
    Curve3(types::Curve3),
    Curve4(types::Curve4),
    BufferInt(types::BufferInt),
    BufferF32(types::BufferF32),
    String256(String),
    Quat(types::Quat),
    U32(u32),
    BufferU32(types::BufferU32),
    BufferBinary(types::BufferBinary),
    StringRef(String),
}

impl Parameter {
    #[inline]
    fn is_string(self: &Parameter) -> bool {
        matches!(
            self,
            Parameter::String32(_)
                | Parameter::String64(_)
                | Parameter::String256(_)
                | Parameter::StringRef(_)
        )
    }

    #[inline]
    fn is_buffer(self: &Parameter) -> bool {
        matches!(
            self,
            Parameter::BufferBinary(_)
                | Parameter::BufferF32(_)
                | Parameter::BufferInt(_)
                | Parameter::BufferU32(_)
        )
    }
}

#[derive(Debug, PartialEq, Clone)]
/// Represents a single AAMP parameter object, containing a map of parameters by hash
pub struct ParameterObject(IndexMap<u32, Parameter>);

impl ParameterObject {
    /// Attempt to get a `Parameter` by name, returns None if not found
    pub fn param(&self, name: &str) -> Option<&Parameter> {
        let mut digest = crc32::Digest::new(crc32::IEEE);
        digest.write(name.as_bytes());
        self.0.get(&digest.sum32())
    }

    /// Sets a parameter value
    pub fn set_param(&mut self, name: &str, value: Parameter) {
        let mut digest = crc32::Digest::new(crc32::IEEE);
        digest.write(name.as_bytes());
        self.0.insert(digest.sum32(), value);
    }
    /// Expose reference to underlying IndexMap
    pub fn params(&self) -> &IndexMap<u32, Parameter> {
        &self.0
    }

    /// Expose mutable reference to underlying IndexMap
    pub fn params_mut(&mut self) -> &mut IndexMap<u32, Parameter> {
        &mut self.0
    }
}

/// Represents a single AAMP parameter list, containing a hash map of parameter objects and
/// child parameter lists
#[derive(Debug, PartialEq, Clone)]
pub struct ParameterList {
    pub lists: IndexMap<u32, ParameterList>,
    pub objects: IndexMap<u32, ParameterObject>,
}

impl ParameterList {
    /// Attempt to get a `ParameterList` by name, returns None if not found
    pub fn list(&self, name: &str) -> Option<&ParameterList> {
        let mut digest = crc32::Digest::new(crc32::IEEE);
        digest.write(name.as_bytes());
        self.lists.get(&digest.sum32())
    }

    /// Attempt to get a `ParameterObject` by name, returns None if not found
    pub fn object(&self, name: &str) -> Option<&ParameterObject> {
        let mut digest = crc32::Digest::new(crc32::IEEE);
        digest.write(name.as_bytes());
        self.objects.get(&digest.sum32())
    }
}

#[derive(Debug, PartialEq, Clone)]
/// Represents a single AAMP parameter IO document
pub struct ParameterIO {
    /// The parameter IO version, required by the format but of no functional importance
    pub version: u32,
    /// The parameter IO type, required by the format but of no functional importance
    pub pio_type: String,
    /// The lists in the parameter IO root list (`param_root`)
    pub lists: IndexMap<u32, ParameterList>,
    /// The objects in the parameter IO root list (`param_root`)
    pub objects: IndexMap<u32, ParameterObject>,
}

impl ParameterIO {
    /// Attempt to get a `ParameterList` by name, returns None if not found
    pub fn list(&self, name: &str) -> Option<&ParameterList> {
        let mut digest = crc32::Digest::new(crc32::IEEE);
        digest.write(name.as_bytes());
        self.lists.get(&digest.sum32())
    }

    /// Attempt to get a `ParameterObject` by name, returns None if not found
    pub fn object(&self, name: &str) -> Option<&ParameterObject> {
        let mut digest = crc32::Digest::new(crc32::IEEE);
        digest.write(name.as_bytes());
        self.objects.get(&digest.sum32())
    }
}

#[cfg(test)]
mod tests {
    use super::ParameterIO;
    use glob::glob;
    use std::fs::File;
    use std::path::PathBuf;

    #[test]
    fn binary_roundtrip() {
        for file in glob("test/**/*.b*").unwrap() {
            let good_file: PathBuf = file.unwrap();
            let mut reader = File::open(&good_file).unwrap();
            let pio: ParameterIO = ParameterIO::from_binary(&mut reader).unwrap();
            let mut cur: std::io::Cursor<Vec<u8>> =
                std::io::Cursor::new(pio.clone().to_binary().unwrap());
            let pio2: ParameterIO = ParameterIO::from_binary(&mut cur).unwrap();
            assert_eq!(pio, pio2);
        }
    }

    #[test]
    fn dump_yaml() {
        for file in glob("test/*.b*").unwrap() {
            let good_file: PathBuf = file.unwrap();
            let mut reader = File::open(&good_file).unwrap();
            let pio: ParameterIO = ParameterIO::from_binary(&mut reader).unwrap();
            pio.to_text().unwrap();
        }
    }

    #[test]
    fn yaml_roundtrip() {
        for file in glob("test/*.b*").unwrap() {
            let good_file: PathBuf = file.unwrap();
            let mut reader = File::open(&good_file).unwrap();
            let pio: ParameterIO = ParameterIO::from_binary(&mut reader).unwrap();
            let new_text = pio.clone().to_text().unwrap();
            let new_pio = ParameterIO::from_text(&new_text).unwrap();
            if pio != new_pio {
                panic!(
                    "{:?} failed YAML roundtrip\n{:?}\n{:?}",
                    &good_file, pio, new_pio
                );
            }
        }
    }

    #[test]
    fn yaml_to_binary() {
        for file in glob("test/*.yml").unwrap().filter_map(|f| f.ok()) {
            let pio = ParameterIO::from_text(&std::fs::read_to_string(&file).unwrap()).unwrap();
            let binary = pio.to_binary().unwrap();
            assert_eq!(
                pio,
                ParameterIO::from_binary(&mut std::io::Cursor::new(binary)).unwrap()
            );
        }
    }
}
