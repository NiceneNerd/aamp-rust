use super::{Parameter, ParameterIO, ParameterList, ParameterObject};
use binwrite::BinWrite;
use indexmap::IndexMap;
use std::error::Error;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

#[derive(Debug, Copy, Clone)]
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

fn get_param_type(param: &Parameter) -> ParameterType {
    match param {
        Parameter::Bool(_) => ParameterType::Bool,
        Parameter::F32(_) => ParameterType::F32,
        Parameter::Int(_) => ParameterType::Int,
        Parameter::Vec2(_) => ParameterType::Vec2,
        Parameter::Vec3(_) => ParameterType::Vec3,
        Parameter::Vec4(_) => ParameterType::Vec4,
        Parameter::Color(_) => ParameterType::Color,
        Parameter::String32(_) => ParameterType::String32,
        Parameter::String64(_) => ParameterType::String64,
        Parameter::Curve1(_) => ParameterType::Curve1,
        Parameter::Curve2(_) => ParameterType::Curve2,
        Parameter::Curve3(_) => ParameterType::Curve3,
        Parameter::Curve4(_) => ParameterType::Curve4,
        Parameter::BufferInt(_) => ParameterType::BufferInt,
        Parameter::BufferF32(_) => ParameterType::BufferF32,
        Parameter::String256(_) => ParameterType::String256,
        Parameter::Quat(_) => ParameterType::Quat,
        Parameter::U32(_) => ParameterType::U32,
        Parameter::BufferU32(_) => ParameterType::BufferU32,
        Parameter::BufferBinary(_) => ParameterType::BufferBinary,
        Parameter::StringRef(_) => ParameterType::StringRef,
    }
}

#[derive(Debug, BinWrite)]
#[binwrite(little)]
struct WriteHeader<'a> {
    magic: &'a [u8; 4],
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

#[derive(Debug, BinWrite)]
#[binwrite(little)]
struct WriteParameterList {
    crc: u32,
    lists_rel_offset: u16,
    num_lists: u16,
    objs_rel_offset: u16,
    num_objs: u16,
}

#[derive(Debug, BinWrite)]
#[binwrite(little)]
struct WriteParameterObject {
    crc: u32,
    params_rel_offset: u16,
    num_params: u16,
}

#[derive(Debug, BinWrite)]
#[binwrite(little)]
struct WriteParameter {
    crc: u32,
    data_offset: [u8; 3],
    #[binwrite(preprocessor(write_param_type))]
    param_type: ParameterType,
}

#[derive(Debug, BinWrite, Clone)]
#[binwrite(little)]
struct WriteParamValue<'a>(&'a Parameter);

#[allow(clippy::trivially_copy_pass_by_ref)]
#[inline]
fn write_param_type(ptype: &ParameterType) -> u8 {
    *ptype as u8
}

#[allow(clippy::trivially_copy_pass_by_ref)]
#[inline]
fn u24_offset(offset: &u32) -> [u8; 3] {
    let bytes = offset.to_le_bytes();
    [bytes[0], bytes[1], bytes[2]]
}

impl ParameterIO {
    /// Serializes an AAMP Parameter IO document to its binary format. Returns a result containing
    /// a `Vec<u8>` or a boxed error.
    pub fn to_binary(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut buffer: Cursor<Vec<u8>> = Cursor::new(vec![]);
        self.write_binary(&mut buffer)?;
        let mut bytes: Vec<u8> = vec![];
        buffer.seek(SeekFrom::Start(0))?;
        buffer.read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    /// Serializes an AAMP Parameter IO document to its binary format using a write implementing the
    /// Write and Seek traits. Returns a result indicating success or a boxed error.
    pub fn write_binary<W: Write + Seek>(&self, writer: &mut W) -> Result<(), Box<dyn Error>> {
        let pio_type = format!("{}\0", self.pio_type);
        let lists_size = (count_lists(&self.lists) + 1) * 12;
        let objs_size = count_objs(&self.lists, self.objects.len()) * 8;
        let params_size = count_params(&self.lists, &self.objects) * 8;
        let mut list_buffer: Cursor<Vec<u8>> = Cursor::new(Vec::with_capacity(lists_size / 12));
        let mut obj_buffer: Cursor<Vec<u8>> = Cursor::new(Vec::with_capacity(objs_size / 8));
        let mut param_buffer: Cursor<Vec<u8>> = Cursor::new(Vec::with_capacity(params_size / 8));
        let mut data_buffer: Cursor<Vec<u8>> = Cursor::new(vec![]);
        WriteParameterList {
            crc: 2_767_637_356,
            lists_rel_offset: 3,
            num_lists: self.lists.len() as u16,
            objs_rel_offset: (lists_size / 4) as u16,
            num_objs: self.objects.len() as u16,
        }
        .write(&mut list_buffer)?;
        let all_params: Vec<(u32, &Parameter)> = write_list_contents(
            0,
            &self.lists,
            &self.objects,
            &mut list_buffer,
            &mut obj_buffer,
            &mut param_buffer,
            lists_size,
            objs_size,
            params_size,
        )?;
        for (offset, param) in all_params.iter().filter(|(_, p)| !p.is_string()) {
            write_param_data(param, *offset as usize, &mut param_buffer, &mut data_buffer)?;
        }
        let data_size = data_buffer.stream_len()? as usize;
        for (offset, param) in all_params.iter().filter(|(_, p)| p.is_string()) {
            write_param_string(param, *offset as usize, &mut param_buffer, &mut data_buffer)?;
        }
        let string_size = data_buffer.stream_len()? as usize - data_size;
        let header = WriteHeader {
            magic: b"AAMP",
            version: 2,
            flags: 3,
            file_size: (0x30
                + align(pio_type.len() as u32) as u64
                + list_buffer.stream_len()?
                + obj_buffer.stream_len()?
                + param_buffer.stream_len()?
                + data_buffer.stream_len()?) as u32,
            pio_version: self.version,
            pio_offset: align(pio_type.len() as u32),
            num_lists: lists_size as u32 / 12,
            num_objects: objs_size as u32 / 8,
            num_params: params_size as u32 / 8,
            data_section_size: data_size as u32,
            string_section_size: string_size as u32,
            idk_section_size: 1,
        };
        header.write(writer)?;
        pio_type.write(writer)?;
        align_cursor(writer)?;
        writer.write_all(list_buffer.get_ref())?;
        writer.write_all(obj_buffer.get_ref())?;
        writer.write_all(param_buffer.get_ref())?;
        align_cursor(writer)?;
        writer.write_all(data_buffer.get_ref())?;
        writer.write_all(&[0])?;
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn write_list_contents<'a>(
    list_offset: u64,
    lists: &'a IndexMap<u32, ParameterList>,
    objects: &'a IndexMap<u32, ParameterObject>,
    list_buffer: &mut Cursor<Vec<u8>>,
    obj_buffer: &mut Cursor<Vec<u8>>,
    param_buffer: &mut Cursor<Vec<u8>>,
    lists_size: usize,
    objs_size: usize,
    params_size: usize,
) -> Result<Vec<(u32, &'a Parameter)>, Box<dyn Error>> {
    let mut all_params: Vec<(u32, &Parameter)> = vec![];
    let pos = list_buffer.stream_position()?;
    if !objects.is_empty() {
        list_buffer.set_position(list_offset + 8);
        list_buffer.write_all(
            &(((obj_buffer.stream_position()? + lists_size as u64 - list_offset) / 4) as u16)
                .to_le_bytes(),
        )?;
        list_buffer.set_position(pos);
        for (crc, obj) in objects.iter() {
            WriteParameterObject {
                crc: *crc,
                params_rel_offset: (((objs_size - obj_buffer.stream_position()? as usize)
                    + param_buffer.stream_position()? as usize)
                    / 4) as u16,
                num_params: obj.0.len() as u16,
            }
            .write(obj_buffer)?;
            for (crc, param) in obj.0.iter() {
                all_params.push((param_buffer.stream_position()? as u32, param));
                WriteParameter {
                    crc: *crc,
                    data_offset: [0, 0, 0],
                    param_type: get_param_type(&param),
                }
                .write(param_buffer)?;
            }
        }
    }
    if !lists.is_empty() {
        let mut offset_map: IndexMap<u32, u64> = IndexMap::new();
        list_buffer.set_position(list_offset + 4);
        list_buffer.write_all(&(((pos - list_offset) / 4) as u16).to_le_bytes())?;
        list_buffer.set_position(pos);
        for (crc, sublist) in lists.iter() {
            offset_map.insert(*crc, list_buffer.stream_position()?);
            WriteParameterList {
                crc: *crc,
                lists_rel_offset: 0,
                num_lists: sublist.lists.len() as u16,
                objs_rel_offset: 0,
                num_objs: sublist.objects.len() as u16,
            }
            .write(list_buffer)?;
        }
        for (crc, sublist) in lists.iter() {
            all_params.extend(write_list_contents(
                offset_map[crc],
                &sublist.lists,
                &sublist.objects,
                list_buffer,
                obj_buffer,
                param_buffer,
                lists_size,
                objs_size,
                params_size,
            )?);
        }
    }
    Ok(all_params)
}

fn write_param_data(
    param: &Parameter,
    parent_offset: usize,
    param_buffer: &mut Cursor<Vec<u8>>,
    data_buffer: &mut Cursor<Vec<u8>>,
) -> Result<(), Box<dyn Error>> {
    let offset_pad = if param.is_buffer() { 4 } else { 0 };
    write_param_offset(
        parent_offset,
        data_buffer.stream_position().unwrap() as u32,
        param_buffer,
        offset_pad,
    )?;
    write_param_value(param, data_buffer)?;
    align_cursor(data_buffer)?;
    Ok(())
}

fn write_param_string(
    param: &Parameter,
    parent_offset: usize,
    param_buffer: &mut Cursor<Vec<u8>>,
    data_buffer: &mut Cursor<Vec<u8>>,
) -> Result<(), Box<dyn Error>> {
    write_param_offset(
        parent_offset,
        data_buffer.stream_position().unwrap() as u32,
        param_buffer,
        0,
    )?;
    write_param_value(param, data_buffer)?;
    align_cursor(data_buffer)?;
    Ok(())
}

fn write_param_offset(
    parent_offset: usize,
    param_offset: u32,
    param_buffer: &mut Cursor<Vec<u8>>,
    pad: usize,
) -> Result<(), Box<dyn Error>> {
    let param_pos = param_buffer.stream_position()?;
    param_buffer.seek(SeekFrom::Start((parent_offset + 4) as u64))?;
    let rel_offset =
        (param_offset as usize + pad + param_buffer.stream_len()? as usize - parent_offset) as u32;
    let red_offset = rel_offset / 4;
    u24_offset(&red_offset).write(param_buffer)?;
    param_buffer.seek(SeekFrom::Start(param_pos))?;
    Ok(())
}

fn count_lists(lists: &IndexMap<u32, ParameterList>) -> usize {
    //&ParameterList) -> usize {
    let sublist_lists: usize = lists
        .values()
        .map(|list: &ParameterList| count_lists(&list.lists))
        .sum();
    lists.len() + sublist_lists
}

fn count_objs(lists: &IndexMap<u32, ParameterList>, objs: usize) -> usize {
    //&ParameterList) -> usize {
    let sublist_objs: usize = lists
        .values()
        .map(|list: &ParameterList| count_objs(&list.lists, list.objects.len()))
        .sum();
    objs + sublist_objs
}

fn count_params(
    lists: &IndexMap<u32, ParameterList>,
    objects: &IndexMap<u32, ParameterObject>,
) -> usize {
    let mut total: usize = 0;
    let sublist_params: usize = lists
        .values()
        .map(|list: &ParameterList| count_params(&list.lists, &list.objects))
        .sum();
    total += sublist_params;
    let obj_params: usize = objects
        .values()
        .map(|obj: &ParameterObject| obj.0.len())
        .sum();
    total += obj_params;
    total
}

#[inline]
fn align_cursor<W: Write + Seek>(buffer: &mut W) -> Result<(), Box<dyn Error>> {
    let pos = buffer.seek(SeekFrom::Current(0))? as u32;
    buffer.seek(SeekFrom::Start(align(pos) as u64))?;
    Ok(())
}

#[inline]
fn align(int: u32) -> u32 {
    int + 4 - 1 - (int - 1) % 4
}

fn write_param_value(
    param: &Parameter,
    buffer: &mut Cursor<Vec<u8>>,
) -> Result<(), Box<dyn Error>> {
    match param {
        Parameter::Bool(b) => (*b as u32).write(buffer)?,
        Parameter::F32(f) => f.write(buffer)?,
        Parameter::Int(i) => i.write(buffer)?,
        Parameter::Vec2(v) => v.0.write(buffer)?,
        Parameter::Vec3(v) => v.0.write(buffer)?,
        Parameter::Vec4(v) => v.0.write(buffer)?,
        Parameter::Color(v) => v.0.write(buffer)?,
        Parameter::String32(s)
        | Parameter::String64(s)
        | Parameter::String256(s)
        | Parameter::StringRef(s) => {
            s.write(buffer)?;
            buffer.write_all(b"\0")?;
        }
        Parameter::Curve1(c) => c.write(buffer)?,
        Parameter::Curve2(c) => c.write(buffer)?,
        Parameter::Curve3(c) => c.write(buffer)?,
        Parameter::Curve4(c) => c.write(buffer)?,
        Parameter::Quat(v) => v.0.write(buffer)?,
        Parameter::U32(u) => u.write(buffer)?,
        Parameter::BufferU32(u) => {
            (u.buffer.len() as u32).write(buffer)?;
            u.write(buffer)?
        }
        Parameter::BufferInt(i) => {
            (i.buffer.len() as u32).write(buffer)?;
            i.write(buffer)?
        }
        Parameter::BufferF32(f) => {
            (f.buffer.len() as u32).write(buffer)?;
            f.write(buffer)?
        }
        Parameter::BufferBinary(b) => {
            (b.buffer.len() as u32).write(buffer)?;
            b.write(buffer)?
        }
    }
    Ok(())
}
