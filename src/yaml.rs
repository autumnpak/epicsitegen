use yaml_rust2::{
    yaml::{Hash, Yaml},
    emitter::{YamlEmitter},
    scanner::ScanError,
    YamlLoader,
};
use yaml_rust2::yaml::Yaml::String as YamlString;
use crate::template::{TemplateError, TemplateValue, TemplateValueAccess};
use crate::utils::{MaybeRef};
use crate::io::FileError;

pub type YamlMap = Hash;
pub type YamlValue = Yaml;

#[derive(Debug, PartialEq, Eq)]
pub enum YamlFileError {
    File(FileError),
    Yaml(ScanError),
}

impl std::fmt::Display for YamlFileError {
    fn fmt(&self, ff: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            YamlFileError::File(err) => err.fmt(ff),
            YamlFileError::Yaml(err) => err.fmt(ff),
        }
    }
}

pub fn load_yaml(strr: &str) -> Result<YamlValue, ScanError> {
    let parsed = YamlLoader::load_from_str(strr);
    match parsed {
        Err(ee) => Err(ee),
        Ok(ss) => Ok(ss[0].clone())
    }
}

pub fn new_yaml_map() -> Hash { Hash::new() }

pub fn lookup_yaml<'a, 'b>(key: &'a str, mapping: &'a YamlMap) -> Result<&'a Yaml, TemplateError> {
    let key_as_yaml = YamlString(key.to_owned());
    match mapping.get(&key_as_yaml) {
        None => Err(TemplateError::KeyNotPresent(key.to_owned())),
        Some(value) => Ok(value),
    }
}

pub fn lookup_str_from_yaml_map<'a, 'b>(key: &'a str, mapping: &'a YamlMap) -> Result<&'a str, TemplateError> {
    let key_as_yaml = YamlString(key.to_owned());
    match mapping.get(&key_as_yaml) {
        None => Err(TemplateError::KeyNotPresent(key.to_owned())),
        Some(value) => match value {
            Yaml::String(ss) => Ok(ss),
            _ => Err(TemplateError::KeyNotString(key.to_owned())),
        }
    }
}

pub fn lookup_value<'a, 'b>(value: &'a TemplateValue, params: &'a YamlMap) -> Result<Yaml, TemplateError> {
    let base = lookup_yaml(&value.base, params)?;
    let mut path: String = value.base.to_owned();
    let mut last_owned = Yaml::Null;
    let mut current = base;
    for aa in &value.accesses {
        match aa {
            TemplateValueAccess::Index(ii) => {
                match current {
                    Yaml::Array(array) => {
                        let index = ii.to_owned();
                        if index >= array.len() {
                            Err(TemplateError::IndexOOB(path.to_owned(), ii.to_owned()))?
                        } else {
                            path = format!("{}[{}]", path, ii);
                            current = &array[index];
                        }
                    },
                    _ => Err(TemplateError::IndexOnUnindexable(path.to_owned(), ii.to_owned()))?,
                }
            } ,
            TemplateValueAccess::Field(ff) => {
                match current {
                    Yaml::Hash(hash) => {
                        match hash.get(&YamlString(ff.to_owned())) {
                            None => Err(TemplateError::FieldNotPresent(path.to_owned(), ff.to_owned()))?,
                            Some(val) => {
                                path = format!("{}.{}", path, ff);
                                current = val;
                            },
                        }
                    },
                    Yaml::Array(aa) => {
                        match lookup_array_property(ff, aa, path.to_owned()) {
                            Err(ee) => Err(ee)?,
                            Ok(val) => {
                                path = format!("{}.{}", path, ff);
                                match val {
                                    MaybeRef::Ref(rr) => current = rr,
                                    MaybeRef::Owned(rr) => {
                                        last_owned = rr;
                                        current = &last_owned;
                                    }
                                }
                            }
                        }
                    },
                    _ => Err(TemplateError::FieldOnUnfieldable(path.to_owned(), ff.to_owned()))?,
                }
            },
        }
    }
    Ok(current.to_owned())
}

pub fn lookup_array_property<'a, 'b>(key: &'a str, arr: &'a Vec<Yaml>, path: String) -> Result<MaybeRef<'a, Yaml>, TemplateError> {
    match key {
        "last" => match arr.last() {
            None => Ok(MaybeRef::Owned(YamlValue::Null)),
            Some(ss) => Ok(MaybeRef::Ref(ss))
        }
        "first" => match arr.first() {
            None => Ok(MaybeRef::Owned(YamlValue::Null)),
            Some(ss) => Ok(MaybeRef::Ref(ss))
        }
        "count" => Ok(MaybeRef::Owned(YamlValue::Integer(arr.len() as i64))),
        _ => Err(TemplateError::FieldNotPresent(path.to_owned(), key.to_owned())),
    }
}

pub fn tostr(value: &Yaml) -> Result<String, TemplateError> {
    let mut outstr = String::new();
    match value {
        Yaml::String(ss) => Ok(ss.clone()),
        Yaml::Real(ss) => Ok(ss.clone()),
        Yaml::Integer(ii) => Ok(format!("{}", ii)),
        _ => match YamlEmitter::new(&mut outstr).dump(value) {
            Ok(_) => {
                Ok(outstr)
            } ,
            Err(ee) => Err(TemplateError::SerialisationError(ee.to_string()))
        }
    }
}

pub fn to_iterable(value: &Yaml) -> Result<Vec<Yaml>, TemplateError> {
    match value {
        Yaml::Array(aa) => Ok(aa.to_owned()),
        _ => Err(TemplateError::ForOnUnindexable("".to_string()))
    }
}

pub fn insert_value(map: &mut Hash, key: &str, value: Yaml) {
    map.insert(YamlString(key.to_string()), value);
}

pub fn unsafe_get_as_string<'a>(map: &'a Hash, key: &str) -> &'a str {
    match map.get(&YamlString(key.to_string())).unwrap() {
        Yaml::String(ss) => ss,
        _ => panic!("unsafe_get_as_string when value wasnt a string")
    }
}
