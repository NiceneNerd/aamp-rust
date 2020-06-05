use super::names;
use super::{Parameter, ParameterIO, ParameterList, ParameterObject};
use libyaml::{Emitter, Event};
use std::error::Error;
use std::io::{BufWriter, Write};

impl ParameterIO {
    pub fn to_text(self: &ParameterIO) -> Result<String, Box<dyn Error>> {
        let mut writer = BufWriter::new(vec![]);
        let mut emit = Emitter::new(writer.by_ref())?;
        emit.emit(Event::StreamStart {
            encoding: Option::Some(libyaml::Encoding::Utf8),
        })?;
        emit.emit(Event::DocumentStart {
            version: None,
            tags: vec![],
            implicit: true,
        })?;
        emit.emit(Event::MappingStart {
            anchor: None,
            tag: Some(String::from("!io")),
            implicit: false,
            style: Some(libyaml::MappingStyle::Block),
        })?;
        emit.emit(Event::Scalar {
            anchor: None,
            tag: None,
            plain_implicit: true,
            quoted_implicit: false,
            style: None,
            value: String::from("version"),
        })?;
        emit.emit(Event::Scalar {
            anchor: None,
            tag: None,
            plain_implicit: true,
            quoted_implicit: false,
            style: None,
            value: format!("{}", self.version),
        })?;
        emit.emit(Event::Scalar {
            anchor: None,
            tag: None,
            plain_implicit: true,
            quoted_implicit: false,
            style: None,
            value: String::from("type"),
        })?;
        emit.emit(Event::Scalar {
            anchor: None,
            tag: None,
            plain_implicit: true,
            quoted_implicit: false,
            style: None,
            value: format!("{}", self.pio_type),
        })?;
        emit.emit(Event::Scalar {
            anchor: None,
            tag: None,
            plain_implicit: true,
            quoted_implicit: false,
            style: None,
            value: String::from("param_root"),
        })?;
        let clone = self.clone();
        let param_root = ParameterList {
            lists: clone.lists,
            objects: clone.objects,
        };
        write_list(2767637356, &param_root, &mut emit)?;
        emit.emit(Event::MappingEnd)?;
        emit.emit(Event::DocumentEnd { implicit: true })?;
        emit.flush()?;
        drop(emit);
        match unescape::unescape(std::str::from_utf8(&writer.into_inner()?)?) {
            Some(s) => Ok(s),
            None => {
                let err: Box<dyn Error> = String::from("Invalid string data").into();
                Err(err)
            }
        }
    }
}

fn write_list(crc: u32, list: &ParameterList, emit: &mut Emitter) -> Result<(), Box<dyn Error>> {
    emit.emit(Event::MappingStart {
        anchor: None,
        tag: Some(String::from("!list")),
        implicit: false,
        style: Some(libyaml::MappingStyle::Block),
    })?;
    emit.emit(Event::Scalar {
        anchor: None,
        tag: None,
        plain_implicit: true,
        quoted_implicit: false,
        style: None,
        value: String::from("objects"),
    })?;
    emit.emit(Event::MappingStart {
        anchor: None,
        tag: None,
        implicit: false,
        style: Some(libyaml::MappingStyle::Block),
    })?;
    for (i, (subcrc, obj)) in list.objects.iter().enumerate() {
        emit.emit(Event::Scalar {
            anchor: None,
            tag: None,
            plain_implicit: true,
            quoted_implicit: false,
            style: None,
            value: try_get_name(&subcrc, &crc, i),
        })?;
        write_object(*subcrc, &obj, emit)?;
    }
    emit.emit(Event::MappingEnd)?;
    emit.emit(Event::Scalar {
        anchor: None,
        tag: None,
        plain_implicit: true,
        quoted_implicit: false,
        style: None,
        value: String::from("lists"),
    })?;
    emit.emit(Event::MappingStart {
        anchor: None,
        tag: None,
        implicit: false,
        style: Some(libyaml::MappingStyle::Block),
    })?;
    for (i, (subcrc, sublist)) in list.lists.iter().enumerate() {
        emit.emit(Event::Scalar {
            anchor: None,
            tag: None,
            plain_implicit: true,
            quoted_implicit: false,
            style: None,
            value: try_get_name(&subcrc, &crc, i),
        })?;
        write_list(crc, &sublist, emit)?;
    }
    emit.emit(Event::MappingEnd)?;
    emit.emit(Event::MappingEnd)?;
    Ok(())
}

fn write_object(crc: u32, obj: &ParameterObject, emit: &mut Emitter) -> Result<(), Box<dyn Error>> {
    emit.emit(Event::MappingStart {
        anchor: None,
        tag: Some(String::from("!obj")),
        implicit: false,
        style: Some(libyaml::MappingStyle::Block),
    })?;
    for (i, (subcrc, param)) in obj.0.iter().enumerate() {
        emit.emit(Event::Scalar {
            anchor: None,
            tag: None,
            plain_implicit: true,
            quoted_implicit: true,
            style: Some(libyaml::ScalarStyle::Plain),
            value: try_get_name(subcrc, &crc, i),
        })?;
        write_param(&param, emit)?;
    }
    emit.emit(Event::MappingEnd)?;
    Ok(())
}

fn write_param(param: &Parameter, emit: &mut Emitter) -> Result<(), Box<dyn Error>> {
    let mut tag: Option<String> = None;
    let mut value = String::new();
    match param {
        Parameter::Bool(b) => value = format!("{:?}", b),
        Parameter::BufferBinary(bb) => {
            write_seq(bb.buffer.iter(), "!buffer_binary", emit)?;
        }
        Parameter::BufferF32(bf) => {
            write_seq(bf.buffer.iter(), "!buffer_f32", emit)?;
        }
        Parameter::BufferInt(bi) => {
            write_seq(bi.buffer.iter(), "!buffer_int", emit)?;
        }
        Parameter::BufferU32(bu) => {
            write_seq(bu.buffer.iter(), "!buffer_u32", emit)?;
        }
        Parameter::Color(c) => {
            write_seq(c.0.iter(), "!color", emit)?;
        }
        Parameter::Curve1(c) => {
            write_seq(curve_to_vec(&c.curve).iter(), "!curve", emit)?;
        }
        Parameter::Curve2(c) => {
            let mut curves = curve_to_vec(&c.curve1);
            curves.extend(curve_to_vec(&c.curve2));
            write_seq(curves.iter(), "!curve", emit)?;
        }
        Parameter::Curve3(c) => {
            let mut curves = curve_to_vec(&c.curve1);
            curves.extend(curve_to_vec(&c.curve2));
            curves.extend(curve_to_vec(&c.curve3));
            write_seq(curves.iter(), "!curve", emit)?;
        }
        Parameter::Curve4(c) => {
            let mut curves = curve_to_vec(&c.curve1);
            curves.extend(curve_to_vec(&c.curve2));
            curves.extend(curve_to_vec(&c.curve3));
            curves.extend(curve_to_vec(&c.curve4));
            write_seq(curves.iter(), "!curve", emit)?;
        }
        Parameter::F32(f) => value = format!("{:?}", f),
        Parameter::Int(i) => value = format!("{}", i),
        Parameter::Quat(q) => write_seq(q.0.iter(), "!quat", emit)?,
        Parameter::String32(s) => {
            value = s.to_string();
            tag = Some(String::from("!str32"))
        }
        Parameter::String64(s) => {
            value = s.to_string();
            tag = Some(String::from("!str64"))
        }
        Parameter::String256(s) => {
            value = s.to_string();
            tag = Some(String::from("!str256"))
        }
        Parameter::StringRef(s) => value = s.to_string(),
        Parameter::U32(u) => {
            value = format!("{}", u);
            tag = Some(String::from("!u"))
        }
        Parameter::Vec2(v) => write_seq(v.0.iter(), "!vec2", emit)?,
        Parameter::Vec3(v) => write_seq(v.0.iter(), "!vec3", emit)?,
        Parameter::Vec4(v) => write_seq(v.0.iter(), "!vec4", emit)?,
    };
    let implicit = match param {
        Parameter::StringRef(_) | Parameter::F32(_) | Parameter::Int(_) | Parameter::Bool(_) => {
            true
        }
        _ => false,
    };
    if !param.is_seq() {
        emit.emit(Event::Scalar {
            anchor: None,
            tag,
            plain_implicit: implicit,
            quoted_implicit: implicit,
            style: if value.len() == 0 {
                Some(libyaml::ScalarStyle::DoubleQuoted)
            } else {
                None
            },
            value,
        })?;
    }
    Ok(())
}

fn write_seq<'a, I, T>(seq: I, tag: &str, emit: &mut Emitter) -> Result<(), Box<dyn Error>>
where
    I: Iterator<Item = &'a T>,
    T: 'a + std::fmt::Display + std::fmt::Debug,
{
    emit.emit(Event::SequenceStart {
        anchor: None,
        tag: Some(String::from(tag)),
        implicit: false,
        style: Some(libyaml::SequenceStyle::Flow),
    })?;
    for i in seq {
        emit.emit(Event::Scalar {
            anchor: None,
            tag: None,
            plain_implicit: true,
            quoted_implicit: false,
            style: None,
            value: format!("{:?}", i),
        })?;
    }
    emit.emit(Event::SequenceEnd)?;
    Ok(())
}

fn curve_to_vec(curve: &super::types::Curve) -> Vec<String> {
    let mut vec = vec![];
    vec.push(format!("{}", curve.a));
    vec.push(format!("{}", curve.b));
    for f in &curve.floats {
        vec.push(format!("{:?}", f));
    }
    vec
}

fn try_get_name(crc: &u32, parent: &u32, idx: usize) -> String {
    match names::get_default_name_table().get_name(*crc) {
        Some(s) => s.to_string(),
        None => match names::guess_name(*crc, *parent, idx) {
            Some(s) => s,
            None => format!("{}", crc),
        },
    }
}
