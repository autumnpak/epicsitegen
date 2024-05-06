use yaml_rust2::{
    yaml::{Hash, Yaml},
    emitter::{YamlEmitter},
};
use yaml_rust2::yaml::Yaml::String as YamlString;
use crate::template::{TemplateError, TemplateValue, TemplateValueAccess};

pub type YamlMap = Hash;

pub fn lookup_yaml_map<'a, 'b>(key: &'a str, mapping: &'a YamlMap) -> Result<&'a Yaml, TemplateError> {
    let key_as_yaml = YamlString(key.to_owned());
    match mapping.get(&key_as_yaml) {
        None => Err(TemplateError::KeyNotPresent(key.to_owned())),
        Some(value) => Ok(value),
    }
}

pub fn lookup_value<'a, 'b>(value: &'a TemplateValue, params: &'a YamlMap) -> Result<&'a Yaml, TemplateError> {
    let base = lookup_yaml_map(&value.base, params)?;
    let mut current: &Yaml = base;
    let mut path: String = value.base.to_owned();
    let mut error: Option<TemplateError> = Option::None;
    for aa in &value.accesses {
        if error.is_none() {
            match aa {
                TemplateValueAccess::Index(ii) => {
                    path = format!("{}[{}]", path, ii);
                    match current {
                        Yaml::Array(array) => {
                            let index = ii.to_owned();
                            if index >= array.len() {
                                error = Some(TemplateError::IndexOOB(path.to_owned(), ii.to_owned()));
                            } else {
                                current = &array[index];
                            }
                        },
                        _ => error = Some(TemplateError::IndexOnUnindexable(path.to_owned(), ii.to_owned())),
                    }
                } ,
                TemplateValueAccess::Field(ff) => {
                    path = format!("{}.{}", path, ff);
                    match current {
                        Yaml::Hash(hash) => {
                            match hash.get(&YamlString(ff.to_owned())) {
                                None => error = Some(TemplateError::FieldNotPresent(path.to_owned(), ff.to_owned())),
                                Some(val) => current = val,
                            }
                        },
                        _ => error = Some(TemplateError::FieldOnUnfieldable(path.to_owned(), ff.to_owned())),
                    }
                },
            }
        }
    }
    if error.is_none() { Ok(current) } else { Err(error.unwrap()) }
}

pub fn tostr(value: &Yaml) -> Result<String, TemplateError> {
    let mut outstr = String::new();
    match value {
        Yaml::String(ss) => Ok(ss.clone()),
        _ => match YamlEmitter::new(&mut outstr).dump(value) {
            Ok(_) => {
                println!("um {}", outstr);
                Ok(outstr)
            } ,
            Err(ee) => Err(TemplateError::SerialisationError(ee.to_string()))
        }
    }
}
