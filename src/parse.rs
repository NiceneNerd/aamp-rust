use super::types;
use super::{Parameter, ParameterIO, ParameterList, ParameterObject};
use binread::{BinRead, NullString};
use indexmap::IndexMap;
use std::convert::TryFrom;
use std::error::Error;
use std::io::{Read, Seek, SeekFrom};

#[derive(Debug)]
pub enum ParseError {
    BinReadError(binread::error::Error),
    Error(Box<dyn Error>),
}

impl From<binread::error::Error> for ParseError {
    fn from(error: binread::error::Error) -> ParseError {
        ParseError::BinReadError(error)
    }
}

impl From<Box<dyn Error>> for ParseError {
    fn from(error: Box<dyn Error>) -> ParseError {
        ParseError::Error(error)
    }
}

impl From<std::io::Error> for ParseError {
    fn from(error: std::io::Error) -> ParseError {
        ParseError::Error(error.into())
    }
}

#[derive(Debug, BinRead)]
enum ParameterType {
    Bool = 0,
    F32,
    Int,
    Vec2,
    Vec3,
    Vec4,
    Color,
    String32,
    String64,
    Curve1,
    Curve2,
    Curve3,
    Curve4,
    BufferInt,
    BufferF32,
    String256,
    Quat,
    U32,
    BufferU32,
    BufferBinary,
    StringRef,
}

impl TryFrom<u8> for ParameterType {
    type Error = String;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ParameterType::Bool),
            1 => Ok(ParameterType::F32),
            2 => Ok(ParameterType::Int),
            3 => Ok(ParameterType::Vec2),
            4 => Ok(ParameterType::Vec3),
            5 => Ok(ParameterType::Vec4),
            6 => Ok(ParameterType::Color),
            7 => Ok(ParameterType::String32),
            8 => Ok(ParameterType::String64),
            9 => Ok(ParameterType::Curve1),
            10 => Ok(ParameterType::Curve2),
            11 => Ok(ParameterType::Curve3),
            12 => Ok(ParameterType::Curve4),
            13 => Ok(ParameterType::BufferInt),
            14 => Ok(ParameterType::BufferF32),
            15 => Ok(ParameterType::String256),
            16 => Ok(ParameterType::Quat),
            17 => Ok(ParameterType::U32),
            18 => Ok(ParameterType::BufferU32),
            19 => Ok(ParameterType::BufferBinary),
            20 => Ok(ParameterType::StringRef),
            _ => Err(format!("Invalid parameter type: {}", value)),
        }
    }
}

#[derive(BinRead, Debug)]
#[br(little, assert(version == 2 && flags & 1 == 1))]
struct ParseHeader {
    version: u32,
    flags: u32,
    file_size: u32,
    pio_version: u32,
    pio_offset: u32,
    num_lists: u32,
    num_objects: u32,
    num_params: u32,
    data_section_size: u32,
    string_section_size: u32,
    idk_section_size: u32,
}

#[derive(BinRead, Debug)]
#[br(little)]
struct ParseParameterList {
    crc: u32,
    lists_rel_offset: u16,
    num_lists: u16,
    objs_rel_offset: u16,
    num_objs: u16,
}

#[derive(BinRead, Debug)]
#[br(little)]
struct ParseParameterObject {
    crc: u32,
    params_rel_offset: u16,
    num_params: u16,
}

#[derive(BinRead, Debug)]
#[br(little)]
struct ParseParameter {
    crc: u32,
    #[br(map = |x: [u8; 3]| u32::from_le_bytes([x[0], x[1], x[2], 0]))]
    data_offset: u32,
    #[br(map = |x: u8| ParameterType::try_from(x).expect(&format!("Invalid type for param {}", crc)))]
    param_type: ParameterType,
}

#[derive(BinRead, Debug)]
#[br(little, magic = b"AAMP")]
struct ParseParameterIO {
    header: ParseHeader,
    pio_type: NullString,
}

#[derive(BinRead, Debug)]
struct ParseBufferInt {
    size: u32,
    #[br(count=size)]
    content: Vec<i32>,
}

#[derive(BinRead, Debug)]
struct ParseBufferF32 {
    size: u32,
    #[br(count=size)]
    content: Vec<f32>,
}

#[derive(BinRead, Debug)]
struct ParseBufferU32 {
    size: u32,
    #[br(count=size)]
    content: Vec<u32>,
}

#[derive(BinRead, Debug)]
struct ParseBufferBinary {
    size: u32,
    #[br(count=size)]
    content: Vec<u8>,
}

impl ParameterIO {
    pub fn from_binary<R: Read + Seek>(reader: &mut R) -> Result<ParameterIO, ParseError> {
        let ppio: ParseParameterIO = ParseParameterIO::read(reader)?;
        reader.seek(SeekFrom::Start((ppio.header.pio_offset + 0x30) as u64))?;
        let parse_pio: ParseParameterList = ParseParameterList::read(reader)?;
        let param_root: ParameterList =
            ParameterList::from_parse_list(parse_pio, ppio.header.pio_offset + 0x30, reader)?;
        let pio = ParameterIO {
            version: ppio.header.pio_version,
            pio_type: ppio.pio_type.to_string(),
            lists: param_root.lists,
            objects: param_root.objects,
        };
        Ok(pio)
    }
}

impl ParameterList {
    fn from_parse_list<R: Read + Seek>(
        plist: ParseParameterList,
        offset: u32,
        reader: &mut R,
    ) -> Result<ParameterList, ParseError> {
        let mut list_map: IndexMap<u32, ParameterList> = IndexMap::new();
        let mut obj_map: IndexMap<u32, ParameterObject> = IndexMap::new();
        if plist.num_lists > 0 {
            for i in 0..plist.num_lists {
                let off = offset + (plist.lists_rel_offset as u32 * 4) + (12 * i as u32);
                reader.seek(SeekFrom::Start(off as u64))?;
                let list: ParseParameterList = ParseParameterList::read(reader)?;
                list_map.insert(list.crc, ParameterList::from_parse_list(list, off, reader)?);
            }
        }
        if plist.num_objs > 0 {
            for i in 0..plist.num_objs {
                let off = offset + (plist.objs_rel_offset as u32 * 4) + (8 * i as u32);
                reader.seek(SeekFrom::Start(off as u64))?;
                let obj: ParseParameterObject = ParseParameterObject::read(reader)?;
                obj_map.insert(obj.crc, ParameterObject::from_parse_obj(obj, off, reader)?);
            }
        }
        Ok(ParameterList {
            lists: list_map,
            objects: obj_map,
        })
    }
}

impl ParameterObject {
    fn from_parse_obj<R: Read + Seek>(
        pobj: ParseParameterObject,
        offset: u32,
        reader: &mut R,
    ) -> Result<ParameterObject, ParseError> {
        let mut param_map: IndexMap<u32, Parameter> = IndexMap::new();
        if pobj.num_params > 0 {
            for i in 0..pobj.num_params {
                let off = offset + (pobj.params_rel_offset as u32 * 4) + (8 * i as u32);
                reader.seek(SeekFrom::Start(off as u64))?;
                let param: ParseParameter = ParseParameter::read(reader)?;
                param_map.insert(
                    param.crc,
                    Parameter::from_parse_param(param, off as u32, reader)?,
                );
            }
        }
        Ok(ParameterObject(param_map))
    }
}

fn add_parsed_string_to_table(string: &str) {
    let mut table = crate::names::TABLE.lock().unwrap();
    table.add_name(string);
}

impl Parameter {
    fn from_parse_param<R: Read + Seek>(
        param: ParseParameter,
        offset: u32,
        reader: &mut R,
    ) -> Result<Parameter, ParseError> {
        let data_offset = offset as u64 + (param.data_offset as u64 * 4);
        reader.seek(SeekFrom::Start(data_offset))?;
        match param.param_type {
            ParameterType::Bool => Ok(Parameter::Bool(u8::read(reader)? == 1)),
            ParameterType::F32 => Ok(Parameter::F32(f32::read(reader)?)),
            ParameterType::Int => Ok(Parameter::Int(i32::read(reader)?)),
            ParameterType::Vec2 => Ok(Parameter::Vec2(types::Vec2::read(reader)?)),
            ParameterType::Vec3 => Ok(Parameter::Vec3(types::Vec3::read(reader)?)),
            ParameterType::Vec4 => Ok(Parameter::Vec4(types::Vec4::read(reader)?)),
            ParameterType::Color => Ok(Parameter::Color(types::Color::read(reader)?)),
            ParameterType::String32 => {
                let name = NullString::read(reader)?.to_string();
                add_parsed_string_to_table(&name);
                Ok(Parameter::String32(name))
            }
            ParameterType::String64 => {
                let name = NullString::read(reader)?.to_string();
                add_parsed_string_to_table(&name);
                Ok(Parameter::String64(name))
            }
            ParameterType::Curve1 => Ok(Parameter::Curve1(types::Curve1::read(reader)?)),
            ParameterType::Curve2 => Ok(Parameter::Curve2(types::Curve2::read(reader)?)),
            ParameterType::Curve3 => Ok(Parameter::Curve3(types::Curve3::read(reader)?)),
            ParameterType::Curve4 => Ok(Parameter::Curve4(types::Curve4::read(reader)?)),
            ParameterType::BufferInt => {
                reader.seek(SeekFrom::Current(-4))?;
                Ok(Parameter::BufferInt(types::BufferInt {
                    buffer: ParseBufferInt::read(reader)?.content,
                }))
            }
            ParameterType::BufferF32 => {
                reader.seek(SeekFrom::Current(-4))?;
                Ok(Parameter::BufferF32(types::BufferF32 {
                    buffer: ParseBufferF32::read(reader)?.content,
                }))
            }
            ParameterType::String256 => {
                let name = NullString::read(reader)?.to_string();
                add_parsed_string_to_table(&name);
                Ok(Parameter::String256(name))
            }
            ParameterType::Quat => Ok(Parameter::Quat(types::Quat::read(reader)?)),
            ParameterType::U32 => Ok(Parameter::U32(u32::read(reader)?)),
            ParameterType::BufferU32 => {
                reader.seek(SeekFrom::Current(-4))?;
                Ok(Parameter::BufferU32(types::BufferU32 {
                    buffer: ParseBufferU32::read(reader)?.content,
                }))
            }
            ParameterType::BufferBinary => {
                reader.seek(SeekFrom::Current(-4))?;
                Ok(Parameter::BufferBinary(types::BufferBinary {
                    buffer: ParseBufferBinary::read(reader)?.content,
                }))
            }
            ParameterType::StringRef => {
                let name = NullString::read(reader)?.to_string();
                add_parsed_string_to_table(&name);
                Ok(Parameter::StringRef(name))
            }
        }
    }
}
