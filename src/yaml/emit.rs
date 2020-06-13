use crate::names;
use crate::{Parameter, ParameterIO, ParameterList, ParameterObject};
use std::error::Error;
use std::io::{BufWriter, Write};

impl ParameterIO {
    /// Returns a YAML representation of an AAMP parameter IO as a string. The output is fully
    /// compatible with the representation used in the `oead` C++ library, and compatible with the
    /// representation used in the `aamp` Python library except where buffer types are used.
    pub fn to_text(self) -> Result<String, Box<dyn Error>> {
        let mut bytes: Vec<u8> = vec![];
        let mut writer = BufWriter::new(&mut bytes);
        self.write_text(&mut writer)?;
        drop(writer);
        Ok(std::str::from_utf8(&mut bytes)?.to_owned())
    }

    /// Writes a YAML document representing an AAMP parameter IO into a writer. The output is fully
    /// compatible with the representation used in the `oead` C++ library, and compatible with the
    /// representation used in the `aamp` Python library except where buffer types are used.
    pub fn write_text<W: Write>(self, writer: &mut W) -> Result<(), Box<dyn Error>> {
        let param_root = ParameterList {
            lists: self.lists,
            objects: self.objects,
        };
        write!(
            writer,
            "!io\nversion: {}\ntype: {}\nparam_root: ",
            self.version, self.pio_type
        )?;
        write_list(writer, &param_root, 2767637356, 1)?;
        writer.flush()?;
        Ok(())
    }
}

const INDENTS: [&str; 20] = [
    "",
    "  ",
    "    ",
    "      ",
    "        ",
    "          ",
    "            ",
    "              ",
    "                ",
    "                  ",
    "                    ",
    "                      ",
    "                        ",
    "                          ",
    "                            ",
    "                              ",
    "                                ",
    "                                  ",
    "                                    ",
    "                                      ",
];

fn write_list<W: Write>(
    writer: &mut W,
    list: &ParameterList,
    crc: u32,
    level: usize,
) -> Result<(), Box<dyn Error>> {
    write!(writer, "!list")?;
    write!(writer, "\n{}objects:", &INDENTS[level])?;
    if list.objects.len() > 0 {
        for (i, (subcrc, obj)) in list.objects.iter().enumerate() {
            write!(
                writer,
                "\n{}{}: ",
                &INDENTS[level + 1],
                try_get_name(subcrc, &crc, i)
            )?;
            write_object(writer, obj, *subcrc, level + 2)?;
        }
    } else {
        write!(writer, " {{}}")?;
    }
    write!(writer, "\n{}lists:", &INDENTS[level])?;
    if list.lists.len() > 0 {
        for (i, (subcrc, sublist)) in list.lists.iter().enumerate() {
            write!(
                writer,
                "\n{}{}: ",
                &INDENTS[level + 1],
                try_get_name(subcrc, &crc, i)
            )?;
            write_list(writer, sublist, *subcrc, level + 2)?;
        }
    } else {
        write!(writer, " {{}}")?;
    }
    Ok(())
}

fn write_object<W: Write>(
    writer: &mut W,
    obj: &ParameterObject,
    crc: u32,
    level: usize,
) -> Result<(), Box<dyn Error>> {
    write!(writer, "!obj")?;
    if obj.0.len() > 0 {
        for (i, (subcrc, param)) in obj.0.iter().enumerate() {
            write!(
                writer,
                "\n{}{}: ",
                &INDENTS[level],
                try_get_name(subcrc, &crc, i)
            )?;
            write_param(writer, param)?;
        }
    } else {
        write!(writer, " {{}}")?;
    }
    Ok(())
}

fn write_param<W: Write>(writer: &mut W, param: &Parameter) -> Result<(), Box<dyn Error>> {
    match param {
        Parameter::Bool(b) => write!(writer, "{}", if *b { "true" } else { "false" })?,
        Parameter::BufferBinary(bb) => {
            write!(writer, "!buffer_binary ")?;
            write_seq(writer, bb.buffer.iter(), bb.buffer.len())?;
        }
        Parameter::BufferF32(bf) => {
            write!(writer, "!buffer_f32 ")?;
            write_float_seq(writer, bf.buffer.iter(), bf.buffer.len())?;
        }
        Parameter::BufferInt(bi) => {
            write!(writer, "!buffer_int ")?;
            write_seq(writer, bi.buffer.iter(), bi.buffer.len())?
        }
        Parameter::BufferU32(bu) => {
            write!(writer, "!buffer_u32 ")?;
            write_seq(writer, bu.buffer.iter(), bu.buffer.len())?
        }
        Parameter::Color(c) => {
            write!(writer, "!color ")?;
            write_float_seq(writer, c.0.iter(), 4)?;
        }
        Parameter::Curve1(c) => {
            write!(writer, "!curve [")?;
            write!(writer, "{}", &curve_to_vec(&c.curve))?;
            write!(writer, "]")?;
        }
        Parameter::Curve2(c) => {
            write!(writer, "!curve [")?;
            write!(writer, "{}", &curve_to_vec(&c.curve1))?;
            write!(writer, ", ")?;
            write!(writer, "{}", &curve_to_vec(&c.curve2))?;
            write!(writer, "]")?;
        }
        Parameter::Curve3(c) => {
            write!(writer, "!curve [")?;
            write!(writer, "{}", &curve_to_vec(&c.curve1))?;
            write!(writer, ", ")?;
            write!(writer, "{}", &curve_to_vec(&c.curve2))?;
            write!(writer, ", ")?;
            write!(writer, "{}", &curve_to_vec(&c.curve3))?;
            write!(writer, "]")?;
        }
        Parameter::Curve4(c) => {
            write!(writer, "!curve [")?;
            write!(writer, "{}", &curve_to_vec(&c.curve1))?;
            write!(writer, ", ")?;
            write!(writer, "{}", &curve_to_vec(&c.curve2))?;
            write!(writer, ", ")?;
            write!(writer, "{}", &curve_to_vec(&c.curve3))?;
            write!(writer, ", ")?;
            write!(writer, "{}", &curve_to_vec(&c.curve4))?;
            write!(writer, "]")?;
        }
        Parameter::F32(f) => write!(writer, "{}", ryu::Buffer::new().format(*f))?,
        Parameter::Int(i) => {
            write!(writer, "{}", i)?;
        }
        Parameter::Quat(q) => {
            write!(writer, "!quat ")?;
            write_float_seq(writer, q.0.iter(), 4)?
        }
        Parameter::String32(s) => {
            write!(writer, "!str32 ")?;
            write_string(writer, s)?
        }
        Parameter::String64(s) => {
            write!(writer, "!str64 ")?;
            write_string(writer, s)?
        }
        Parameter::String256(s) => {
            write!(writer, "!str256 ")?;
            write_string(writer, s)?
        }
        Parameter::StringRef(s) => write_string(writer, s)?,
        Parameter::U32(u) => write!(writer, "!u 0x{:X}", u)?,
        Parameter::Vec2(v) => {
            write!(writer, "!vec2 ")?;
            write_float_seq(writer, v.0.iter(), 2)
        }?,
        Parameter::Vec3(v) => {
            write!(writer, "!vec3 ")?;
            write_float_seq(writer, v.0.iter(), 3)
        }?,
        Parameter::Vec4(v) => {
            write!(writer, "!vec4 ")?;
            write_float_seq(writer, v.0.iter(), 4)
        }?,
    };
    Ok(())
}

fn write_string<W: Write>(writer: &mut W, string: &str) -> Result<(), Box<dyn Error>> {
    if string.contains(" ") || parse_int::parse::<usize>(string).is_ok() || string.len() == 0 {
        write!(writer, "\"")?;
        write!(writer, "{}", string)?;
        write!(writer, "\"")?;
    } else {
        write!(writer, "{}", string)?;
    }
    Ok(())
}

fn write_seq<'a, I, T, W>(writer: &mut W, seq: I, count: usize) -> Result<(), Box<dyn Error>>
where
    I: Iterator<Item = &'a T>,
    T: 'a + num_format::ToFormattedStr + std::fmt::Display,
    W: Write,
{
    write!(writer, "[")?;
    for (i, x) in seq.enumerate() {
        write!(writer, "{}", x)?;
        if i < count - 1 {
            write!(writer, ", ")?;
        }
    }
    write!(writer, "]")?;
    Ok(())
}

fn write_float_seq<'a, I, T, W>(writer: &mut W, seq: I, count: usize) -> Result<(), Box<dyn Error>>
where
    I: Iterator<Item = &'a T>,
    T: 'a + ryu::Float,
    W: Write,
{
    let mut buf = ryu::Buffer::new();
    write!(writer, "[")?;
    for (i, x) in seq.enumerate() {
        write!(writer, "{}", buf.format(*x))?;
        if i < count - 1 {
            write!(writer, ", ")?;
        }
    }
    write!(writer, "]")?;
    Ok(())
}

fn curve_to_vec(curve: &crate::types::Curve) -> String {
    let mut vec = Vec::with_capacity(3);
    vec.push(format!("{}", curve.a));
    vec.push(format!("{}", curve.b));
    let mut buf = ryu::Buffer::new();
    vec.push(
        curve
            .floats
            .iter()
            .map(|f| buf.format(*f).to_string())
            .collect::<Vec<String>>()
            .join(", "),
    );
    vec.join(", ")
}

fn try_get_name(crc: &u32, parent: &u32, idx: usize) -> String {
    let table = names::TABLE.lock().unwrap();
    match table.get_name(*crc) {
        Some(s) => match s.parse::<u32>() {
            Ok(s) => format!("\"{}\"", s),
            Err(_) => s.to_string(),
        },
        None => {
            drop(table);
            match names::guess_name(*crc, *parent, idx) {
                Some(s) => match s.parse::<u32>() {
                    Ok(s) => format!("\"{}\"", s),
                    Err(_) => s.to_string(),
                },
                None => format!("{}", crc),
            }
        }
    }
}
