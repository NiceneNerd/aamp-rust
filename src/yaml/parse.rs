use super::forked::parser::*;
use super::forked::scanner::*;
use crate::types::*;
use crate::{Parameter, ParameterIO, ParameterList, ParameterObject};
use crc::{crc32, Hasher32};
use indexmap::IndexMap;
use thiserror::Error;

type Result<T> = std::result::Result<T, YamlParseError>;

#[derive(Debug, Error)]
pub enum YamlParseError {
    #[error("YAML document not a valid ParameterIO: {0}")]
    InvalidPio(String),
    #[error("YAML document is not valid")]
    InvalidYaml(#[from] crate::yaml::forked::scanner::ScanError),
    #[error("YAML contains invalid integer: {0}")]
    InvalidInt(#[from] std::num::ParseIntError),
    #[error("YAML has invalid float: {0}")]
    InvalidFloat(#[from] std::num::ParseFloatError),
}

impl ParameterIO {
    /// Parses an AAMP Parameter IO document from a YAML representation. Takes a string slice and
    /// returns a result containing a `ParameterIO` or a boxed error.
    pub fn from_text(text: &str) -> Result<ParameterIO> {
        let mut parser = Parser::new(text.chars());
        let (pio_type, version) = parse_header(&mut parser)?;
        let mut pio_parser = PioYamlParser::new(version, pio_type);
        let next = parser.next()?;
        parser.load_node(next.0, next.1, &mut pio_parser)?;
        match pio_parser.error {
            Some(err) => Err(err),
            None => match pio_parser.pio {
                Some(pio) => Ok(pio),
                None => Err(YamlParseError::InvalidPio(
                    "Could not parse document".to_owned(),
                )),
            },
        }
    }
}

fn parse_header<T: Iterator<Item = char>>(parser: &mut Parser<T>) -> Result<(String, u32)> {
    match parser.next()?.0 {
        Event::StreamStart => (),
        _ => return Err(YamlParseError::InvalidPio("No stream start".to_owned())),
    };
    match parser.next()?.0 {
        Event::DocumentStart => (),
        _ => return Err(YamlParseError::InvalidPio("No doc start".to_owned())),
    };
    match parser.next()?.0 {
        Event::MappingStart(_, tag) => match tag {
            Some(TokenType::Tag(ref _handle, ref suffix)) => {
                assert_eq!(suffix.as_str(), "io");
                match parser.next()?.0 {
                    Event::Scalar(v, _, _, _) => {
                        assert_eq!(&v, "version");
                        match parser.next()?.0 {
                            Event::Scalar(v, _, _, _) => {
                                let version = v.parse::<u32>()?;
                                match parser.next()?.0 {
                                    Event::Scalar(v, _, _, _) => {
                                        assert_eq!(&v, "type");
                                        match parser.next()?.0 {
                                            Event::Scalar(v, _, _, _) => {
                                                let pio_type = v;
                                                match parser.next()?.0 {
                                                    Event::Scalar(v, _, _, _) => {
                                                        assert_eq!(&v, "param_root");
                                                        Ok((pio_type, version))
                                                    }
                                                    _ => Err(YamlParseError::InvalidPio(
                                                        "Missing param root".to_owned(),
                                                    )),
                                                }
                                            }
                                            _ => Err(YamlParseError::InvalidPio(
                                                "Missing type".to_owned(),
                                            )),
                                        }
                                    }
                                    _ => Err(YamlParseError::InvalidPio("Missing type".to_owned())),
                                }
                            }
                            _ => Err(YamlParseError::InvalidPio("Missing version".to_owned())),
                        }
                    }
                    _ => Err(YamlParseError::InvalidPio("Missing version".to_owned())),
                }
            }
            _ => Err(YamlParseError::InvalidPio(
                "Not a Parameter IO document".to_owned(),
            )),
        },
        _ => Err(YamlParseError::InvalidPio("No mapping start".to_owned())),
    }
}

struct PioYamlParser {
    open_seq: Option<Vec<String>>,
    open_tag: Option<String>,
    open_keys: Vec<String>,
    open_params: Option<IndexMap<u32, Parameter>>,
    open_objs: Vec<IndexMap<u32, ParameterObject>>,
    open_list_maps: Vec<IndexMap<u32, ParameterList>>,
    open_lists: Vec<ParameterList>,
    doing_objects: bool,
    doing_lists: bool,
    doing_param_key: bool,
    pio_type: String,
    pio_version: u32,
    pio: Option<ParameterIO>,
    error: Option<YamlParseError>,
    last_event: Option<Event>,
}

impl MarkedEventReceiver for PioYamlParser {
    fn on_event(&mut self, ev: Event, mark: Marker) {
        if self.error.is_some() {
            return;
        }
        let okay = || -> Result<()> {
            match ev.clone() {
                Event::MappingStart(_, tag) => {
                    match tag {
                        Some(TokenType::Tag(ref _handle, ref suffix)) => match suffix.as_str() {
                            "list" => {
                                self.open_lists.push(ParameterList {
                                    lists: IndexMap::new(),
                                    objects: IndexMap::new(),
                                });
                            }
                            "obj" => {
                                self.open_params = Some(IndexMap::new());
                                self.doing_param_key = true;
                            }
                            _ => {
                                return Err(YamlParseError::InvalidPio(format!(
                                    "Bad mapping tag at {:?}",
                                    mark
                                )))
                            }
                        },
                        _ => {
                            if !(self.doing_lists || self.doing_objects) {
                                return Err(YamlParseError::InvalidPio(format!(
                                    "Bad mapping tag at {:?}",
                                    mark
                                )));
                            }
                        }
                    };
                }
                Event::MappingEnd => {
                    if self.open_params.is_some() {
                        let params = self
                            .open_params
                            .take()
                            .ok_or_else(|| YamlParseError::InvalidPio("No params".to_owned()))?;
                        let key = self
                            .open_keys
                            .pop()
                            .ok_or_else(|| YamlParseError::InvalidPio("No keys".to_owned()))?;
                        self.open_objs
                            .last_mut()
                            .ok_or_else(|| YamlParseError::InvalidPio("No objcts".to_owned()))?
                            .insert(hashit(&key), ParameterObject(params));
                        self.doing_param_key = false;
                    } else if self.doing_objects {
                        self.doing_objects = false;
                        self.last_event = None;
                        self.doing_lists = true;
                    } else if let Some(Event::MappingStart(..)) = self.last_event {
                        self.doing_lists = true;
                    } else if let Some(Event::MappingEnd) = self.last_event {
                        if self.doing_lists && self.open_list_maps.len() > 1 {
                            let list_map = self
                                .open_list_maps
                                .pop()
                                .ok_or_else(|| YamlParseError::InvalidPio("No lists".to_owned()))?;
                            let obj_map = self.open_objs.pop().ok_or_else(|| {
                                YamlParseError::InvalidPio("No objects".to_owned())
                            })?;
                            let key = self
                                .open_keys
                                .pop()
                                .ok_or_else(|| YamlParseError::InvalidPio("No keys".to_owned()))?;
                            self.open_list_maps
                                .last_mut()
                                .ok_or_else(|| {
                                    YamlParseError::InvalidPio("No list maps".to_owned())
                                })?
                                .insert(
                                    hashit(&key),
                                    ParameterList {
                                        lists: list_map,
                                        objects: obj_map,
                                    },
                                );
                        } else if self.open_list_maps.len() == 1 {
                            self.pio = Some(ParameterIO {
                                pio_type: self.pio_type.to_owned(),
                                version: self.pio_version,
                                lists: self.open_list_maps.pop().ok_or_else(|| {
                                    YamlParseError::InvalidPio("No list maps".to_owned())
                                })?,
                                objects: self.open_objs.pop().ok_or_else(|| {
                                    YamlParseError::InvalidPio("No objects".to_owned())
                                })?,
                            })
                        }
                        self.doing_lists = !self.doing_lists;
                    }
                }
                Event::SequenceStart(_, tag) => {
                    self.open_seq = Some(vec![]);
                    match tag {
                        Some(TokenType::Tag(ref _handle, ref suffix)) => {
                            self.open_tag = Some(suffix.to_owned())
                        }
                        _ => {
                            return Err(YamlParseError::InvalidPio(format!(
                                "Missing sequence tag at {:?}",
                                mark
                            )))
                        }
                    }
                }
                Event::SequenceEnd => {
                    let seq = self
                        .open_seq
                        .take()
                        .ok_or_else(|| YamlParseError::InvalidPio("No sequence".to_owned()))?;
                    let tag = self
                        .open_tag
                        .take()
                        .ok_or_else(|| YamlParseError::InvalidPio("No sequence tag".to_owned()))?;
                    let param: Parameter = match tag.as_str() {
                        "vec2" => {
                            Parameter::Vec2(Vec2([seq[0].parse::<f32>()?, seq[1].parse::<f32>()?]))
                        }
                        "vec3" => Parameter::Vec3(Vec3([
                            seq[0].parse::<f32>()?,
                            seq[1].parse::<f32>()?,
                            seq[2].parse::<f32>()?,
                        ])),
                        "vec4" => Parameter::Vec4(Vec4([
                            seq[0].parse::<f32>()?,
                            seq[1].parse::<f32>()?,
                            seq[2].parse::<f32>()?,
                            seq[3].parse::<f32>()?,
                        ])),
                        "quat" => Parameter::Quat(Quat([
                            seq[0].parse::<f32>()?,
                            seq[1].parse::<f32>()?,
                            seq[2].parse::<f32>()?,
                            seq[3].parse::<f32>()?,
                        ])),
                        "color" => Parameter::Color(Color([
                            seq[0].parse::<f32>()?,
                            seq[1].parse::<f32>()?,
                            seq[2].parse::<f32>()?,
                            seq[3].parse::<f32>()?,
                        ])),
                        "curve" => vec_to_curve(seq)?,
                        "buffer_int" => Parameter::BufferInt(BufferInt {
                            buffer: seq
                                .iter()
                                .map(|x| x.parse::<i32>().map_err(|e| e.into()))
                                .collect::<Result<Vec<i32>>>()?,
                        }),
                        "buffer_u32" => Parameter::BufferU32(BufferU32 {
                            buffer: seq
                                .iter()
                                .map(|x| parse_int::parse::<u32>(&x).map_err(|e| e.into()))
                                .collect::<Result<Vec<u32>>>()?,
                        }),
                        "buffer_binary" => Parameter::BufferBinary(BufferBinary {
                            buffer: seq
                                .iter()
                                .map(|x| parse_int::parse::<u8>(&x).map_err(|e| e.into()))
                                .collect::<Result<Vec<u8>>>()?,
                        }),
                        "buffer_f32" => Parameter::BufferF32(BufferF32 {
                            buffer: seq
                                .iter()
                                .map(|x| x.parse::<f32>().map_err(|e| e.into()))
                                .collect::<Result<Vec<f32>>>()?,
                        }),
                        _ => return Err(YamlParseError::InvalidPio("Unknown type tag".to_owned())),
                    };
                    match &self.open_keys.pop() {
                        Some(key) => {
                            self.open_params
                                .as_mut()
                                .ok_or_else(|| YamlParseError::InvalidPio("No params".to_owned()))?
                                .insert(hashit(key), param);
                        }
                        _ => return Err(YamlParseError::InvalidPio("No key for value".to_owned())),
                    }
                    self.doing_param_key = true;
                    return Ok(());
                }
                Event::Scalar(value, style, _, tag) => {
                    self.read_scalar(value, style, tag);
                }
                _ => {}
            };
            self.last_event = Some(ev);
            Ok(())
        };
        match okay() {
            Ok(_) => (),
            Err(err) => self.error = Some(err),
        };
    }
}

fn vec_to_curve(seq: Vec<String>) -> Result<Parameter> {
    assert_eq!(seq.len() % 32, 0);
    Ok(match seq.len() / 32 {
        1 => Parameter::Curve1(Curve1 {
            curve: Curve {
                a: seq[0].parse::<u32>()?,
                b: seq[1].parse::<u32>()?,
                floats: seq[2..]
                    .iter()
                    .map(|x| x.parse::<f32>().map_err(|e| e.into()))
                    .collect::<Result<Vec<f32>>>()?,
            },
        }),
        2 => Parameter::Curve2(Curve2 {
            curve1: Curve {
                a: seq[0].parse::<u32>()?,
                b: seq[1].parse::<u32>()?,
                floats: seq[2..32]
                    .iter()
                    .map(|x| x.parse::<f32>().map_err(|e| e.into()))
                    .collect::<Result<Vec<f32>>>()?,
            },
            curve2: Curve {
                a: seq[32].parse::<u32>()?,
                b: seq[33].parse::<u32>()?,
                floats: seq[34..64]
                    .iter()
                    .map(|x| x.parse::<f32>().map_err(|e| e.into()))
                    .collect::<Result<Vec<f32>>>()?,
            },
        }),
        3 => Parameter::Curve3(Curve3 {
            curve1: Curve {
                a: seq[0].parse::<u32>()?,
                b: seq[1].parse::<u32>()?,
                floats: seq[2..32]
                    .iter()
                    .map(|x| x.parse::<f32>().map_err(|e| e.into()))
                    .collect::<Result<Vec<f32>>>()?,
            },
            curve2: Curve {
                a: seq[32].parse::<u32>()?,
                b: seq[33].parse::<u32>()?,
                floats: seq[34..64]
                    .iter()
                    .map(|x| x.parse::<f32>().map_err(|e| e.into()))
                    .collect::<Result<Vec<f32>>>()?,
            },
            curve3: Curve {
                a: seq[64].parse::<u32>()?,
                b: seq[65].parse::<u32>()?,
                floats: seq[66..96]
                    .iter()
                    .map(|x| x.parse::<f32>().map_err(|e| e.into()))
                    .collect::<Result<Vec<f32>>>()?,
            },
        }),
        4 => Parameter::Curve4(Curve4 {
            curve1: Curve {
                a: seq[0].parse::<u32>()?,
                b: seq[1].parse::<u32>()?,
                floats: seq[2..32]
                    .iter()
                    .map(|x| x.parse::<f32>().map_err(|e| e.into()))
                    .collect::<Result<Vec<f32>>>()?,
            },
            curve2: Curve {
                a: seq[32].parse::<u32>()?,
                b: seq[33].parse::<u32>()?,
                floats: seq[34..64]
                    .iter()
                    .map(|x| x.parse::<f32>().map_err(|e| e.into()))
                    .collect::<Result<Vec<f32>>>()?,
            },
            curve3: Curve {
                a: seq[64].parse::<u32>()?,
                b: seq[65].parse::<u32>()?,
                floats: seq[66..96]
                    .iter()
                    .map(|x| x.parse::<f32>().map_err(|e| e.into()))
                    .collect::<Result<Vec<f32>>>()?,
            },
            curve4: Curve {
                a: seq[96].parse::<u32>()?,
                b: seq[97].parse::<u32>()?,
                floats: seq[98..128]
                    .iter()
                    .map(|x| x.parse::<f32>().map_err(|e| e.into()))
                    .collect::<Result<Vec<f32>>>()?,
            },
        }),
        _ => panic!("Invalid curve length"),
    })
}

impl PioYamlParser {
    fn new(version: u32, pio_type: String) -> PioYamlParser {
        PioYamlParser {
            pio_type,
            pio_version: version,
            doing_objects: false,
            doing_lists: true,
            doing_param_key: false,
            open_seq: None,
            open_tag: None,
            open_params: None,
            open_keys: vec![String::from("param_root")],
            open_lists: vec![],
            open_objs: vec![],
            open_list_maps: vec![],
            error: None,
            pio: None,
            last_event: None,
        }
    }

    fn read_scalar(&mut self, val: String, style: TScalarStyle, tag: Option<TokenType>) {
        let okay = || -> Result<()> {
            if let Some(seq) = self.open_seq.as_mut() {
                seq.push(val);
                Ok(())
            } else {
                let mut table = crate::names::TABLE.lock().unwrap();
                if let Some(params) = self.open_params.as_mut() {
                    if !self.doing_param_key {
                        let param: Parameter = match tag {
                            Some(TokenType::Tag(ref _handle, ref suffix)) => {
                                match suffix.as_str() {
                                    "str32" => {
                                        table.add_name(&val);
                                        Parameter::String32(val)
                                    }
                                    "str64" => {
                                        table.add_name(&val);
                                        Parameter::String64(val)
                                    }
                                    "str256" => {
                                        table.add_name(&val);
                                        Parameter::String256(val)
                                    }
                                    "u" => Parameter::U32(parse_int::parse::<u32>(&val)?),
                                    _ => {
                                        table.add_name(&val);
                                        Parameter::StringRef(val)
                                    }
                                }
                            }
                            _ => match style {
                                TScalarStyle::SingleQuoted | TScalarStyle::DoubleQuoted => {
                                    Parameter::StringRef(val)
                                }
                                _ => match val.parse::<i32>() {
                                    Ok(v) => Parameter::Int(v),
                                    Err(_) => match val.parse::<f32>() {
                                        Ok(v) => Parameter::F32(v),
                                        Err(_) => match val.as_str() {
                                            "true" => Parameter::Bool(true),
                                            "false" => Parameter::Bool(false),
                                            _ => {
                                                table.add_name(&val);
                                                Parameter::StringRef(val)
                                            }
                                        },
                                    },
                                },
                            },
                        };
                        match &self.open_keys.pop() {
                            Some(key) => {
                                params.insert(hashit(key), param);
                            }
                            None => {
                                return Err(YamlParseError::InvalidPio(
                                    "No key for value".to_owned(),
                                ))
                            }
                        }
                        self.doing_param_key = true;
                        return Ok(());
                    }
                }
                match val.as_str() {
                    "objects" => {
                        self.doing_objects = true;
                        self.open_objs.push(IndexMap::new());
                    }
                    "lists" => {
                        self.doing_lists = true;
                        self.open_list_maps.push(IndexMap::new());
                    }
                    _ => {
                        self.doing_param_key = false;
                        match style {
                            TScalarStyle::DoubleQuoted | TScalarStyle::SingleQuoted => {
                                match val.parse::<u32>() {
                                    Ok(u) => {
                                        self.open_keys.push(["\"", &u.to_string(), "\""].join(""))
                                    }
                                    Err(_) => self.open_keys.push(val),
                                }
                            }
                            _ => self.open_keys.push(val),
                        }
                    }
                };
                Ok(())
            }
        };
        match okay() {
            Ok(_) => (),
            Err(e) => self.error = Some(e),
        }
    }
}

#[inline]
fn hashit(string: &str) -> u32 {
    return match string.parse::<u32>() {
        Ok(crc) => crc,
        Err(_) => {
            let unquoted = string.replace("\"", "");
            do_hash(&unquoted)
        }
    };
    #[inline(always)]
    fn do_hash(string: &str) -> u32 {
        let mut digest = crc32::Digest::new(crc32::IEEE);
        digest.write(string.as_bytes());
        digest.sum32()
    }
}
