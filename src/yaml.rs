use serde_yaml::{Value, Mapping, to_string};
use crate::template::{TemplateError};

pub fn lookup_yaml_map<'a, 'b>(mapping: &'a Mapping, key: &str) -> Result<&'a Value, TemplateError> {
    match mapping.get(key) {
        None => Err(TemplateError::KeyNotPresent(key.to_owned())),
        Some(value) => Ok(value),
    }
}

pub fn tostr(value: &Value) -> Result<String, TemplateError> {
    to_string(value).map_err(|ee| TemplateError::SerialisationError(ee.to_string()))
}
