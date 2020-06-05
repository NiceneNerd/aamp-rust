#![feature(seek_convenience, const_fn)]
use indexmap::IndexMap;
pub mod names;
mod parse;
pub mod types;
mod write;
mod yaml;

#[derive(Debug, PartialEq, Clone)]
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
    fn is_string(self: &Parameter) -> bool {
        match self {
            Parameter::String32(_)
            | Parameter::String64(_)
            | Parameter::String256(_)
            | Parameter::StringRef(_) => true,
            _ => false,
        }
    }

    fn is_buffer(self: &Parameter) -> bool {
        match self {
            Parameter::BufferBinary(_)
            | Parameter::BufferF32(_)
            | Parameter::BufferInt(_)
            | Parameter::BufferU32(_) => true,
            _ => false,
        }
    }

    fn is_seq(self: &Parameter) -> bool {
        if self.is_string() {
            return false;
        };
        if self.is_buffer() {
            return true;
        };
        match self {
            Parameter::Bool(_) | Parameter::F32(_) | Parameter::U32(_) | Parameter::Int(_) => false,
            _ => true,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ParameterObject(IndexMap<u32, Parameter>);

#[derive(Debug, PartialEq, Clone)]
pub struct ParameterList {
    pub lists: IndexMap<u32, ParameterList>,
    pub objects: IndexMap<u32, ParameterObject>,
}

#[derive(Debug, PartialEq, Clone)]
/// Represents a single AAMP ParameterIO document
pub struct ParameterIO {
    /// The ParameterIO version, required by the format but of no functional importance
    pub version: u32,
    /// The ParameterIO type, required by the format but of no functional importance
    pub pio_type: String,
    /// The lists in the ParameterIO root list (`param_root`)
    pub lists: IndexMap<u32, ParameterList>,
    /// The objects in the ParameterIO root list (`param_root`)
    pub objects: IndexMap<u32, ParameterObject>,
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
}
