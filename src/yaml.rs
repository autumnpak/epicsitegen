use yaml_rust2::{
    yaml::{Array, Hash, Yaml},
    emitter::{YamlEmitter},
};
use yaml_rust2::yaml::Yaml::String as YamlString;
use crate::template::{TemplateError};

pub type YamlMap = Hash;

pub fn lookup_yaml_map<'a, 'b>(mapping: &'a YamlMap, key: &str) -> Result<&'a Yaml, TemplateError> {
    let key_as_yaml = YamlString(key.to_owned());
    match mapping.get(&key_as_yaml) {
        None => Err(TemplateError::KeyNotPresent(key.to_owned())),
        Some(value) => Ok(value),
    }
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
