use crate::yaml::{YamlValue, YamlMap, load_yaml, YamlFileError, new_yaml_map};
use crate::utils::{map_m, map_m_ref_index, map_m_index};
use crate::build::{BuildAction, BuildMultiplePages};
use crate::io::{ReadsFiles};
use yaml_rust2::scanner::ScanError;

#[derive(Debug, PartialEq, Eq)]
pub enum LayoutError {
    AtEntry(Box<LayoutError>, usize),
    YamlParsing(ScanError),
    YamlFileError(YamlFileError),
    UnexpectedType(String),
    MissingKey(String),
    KeyNotString(String),
    KeyNotArray(String),
    KeyNotMap(String),
    EntryNotHash(String, usize),
    EntryNotString(String, usize),
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, ff: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LayoutError::AtEntry(ee, ind) => write!(ff, "At action {}: {}", ind, ee),
            LayoutError::YamlParsing(ee) => ee.fmt(ff),
            LayoutError::YamlFileError(ee) => ee.fmt(ff),
            LayoutError::UnexpectedType(ee) => write!(ff, "\"{}\" is not a valid type of build action", ee),
            LayoutError::MissingKey(ee) => write!(ff, "The key \"{}\" is missing", ee),
            LayoutError::KeyNotString(ee) => write!(ff, "The key \"{}\" is not a string", ee),
            LayoutError::KeyNotArray(ee) => write!(ff, "The key \"{}\" is not an array", ee),
            LayoutError::KeyNotMap(ee) => write!(ff, "The key \"{}\" is not a map", ee),
            LayoutError::EntryNotHash(ee, pos) => write!(ff, "Entry {} within the array at \"{}\" is not a map", pos, ee),
            LayoutError::EntryNotString(ee, pos) => write!(ff, "Entry {} within the array at \"{}\" is not a string", pos, ee),
        }
    }
}

pub fn layout_file_to_buildactions(
    filename: &str,
    io: &mut impl ReadsFiles,
) -> Result<Vec<BuildAction>, LayoutError> {
    let file = io.read_yaml(filename).map_err(|ee| LayoutError::YamlFileError(ee))?;
    layout_file_parsed_to_buildactions(file)
}

pub fn layout_string_to_buildactions(
    contents: &str,
) -> Result<Vec<BuildAction>, LayoutError> {
    let file = load_yaml(contents).map_err(|ee| LayoutError::YamlParsing(ee))?;
    layout_file_parsed_to_buildactions(&file)
}

pub fn layout_file_parsed_to_buildactions(
    actions: &YamlValue,
) -> Result<Vec<BuildAction>, LayoutError> {
    let arr = ensure_array_of_hash(Ok(actions), "(base value)")?;
    map_m_index(arr, |ind, aa| yaml_map_to_buildaction(aa).map_err(|ee| LayoutError::AtEntry(Box::new(ee), ind)))
}

pub fn yaml_map_to_buildaction<'a>(
    mapping: &'a YamlMap
) -> Result<BuildAction, LayoutError> {
    let actiontype = lookup_yaml_str("type", mapping)?;
    match actiontype {
        "copy" => {
            let from = lookup_yaml_str("from", mapping)?;
            let to = lookup_yaml_str("to", mapping)?;
            Ok(BuildAction::CopyFiles{from: from.to_owned(), to: to.to_owned()})
        },
        "build" => {
            let input = lookup_yaml_str("input", mapping)?;
            let output = lookup_yaml_str("output", mapping)?;
            let params = lookup_yaml_hash("params", mapping)?;
            Ok(BuildAction::BuildPage{
                input: input.to_owned(), output: output.to_owned(), params: params.to_owned()
            })
        },
        "build-multiple" => {
            let descriptor = lookup_yaml_str("description", mapping)?;
            let default = lookup_yaml_hash("default", mapping)?;
            let with_value_raw = lookup_yaml("with", mapping);
            let with_value = ensure_array_of_hash(with_value_raw, "with")?;
            let include = lookup_yaml_str("include", mapping).ok().map(|ii| ii.to_owned());
            let exclude = lookup_yaml_str("exclude", mapping).ok().map(|ii| ii.to_owned());
            let withs = map_m(with_value, |ii| {
                let filesraw = lookup_yaml("files", ii);
                let files = ensure_array_of_string(filesraw, "files")?;
                let paramsraw = lookup_yaml("params", ii);
                let mut params: Vec<YamlMap> = Vec::new();
                for pp in ensure_array_of_hash(paramsraw, "params")? {
                    params.push(pp.clone());
                };
                let mapping_map: YamlMap = match lookup_yaml_hash("mapping", ii) {
                    Ok(aa) => Ok(aa.to_owned()),
                    Err(LayoutError::MissingKey(_)) => Ok(new_yaml_map()),
                    Err(ee) => Err(ee)
                }?;
                let flatten = lookup_yaml_str("flatten", ii).ok().map(|ii| ii.to_owned());
                Ok(BuildMultiplePages{
                    files: files.to_owned(),
                    params: params,
                    mapping: mapping_map,
                    flatten,
                })
            })?;
            Ok(BuildAction::BuildMultiplePages{
                on: withs, include, exclude,
                default_params: default.to_owned(), descriptor: descriptor.to_owned()
            })
        },
        _ => Err(LayoutError::UnexpectedType(actiontype.to_owned())),
    }
}

fn ensure_array_of_hash<'a, 'b>(
    value: Result<&'a YamlValue, LayoutError>, key: &'a str
) -> Result<Vec<&'a YamlMap>, LayoutError> {
    match value {
        Ok(YamlValue::Array(aa)) => {
            let mut result: Vec<&'a YamlMap> = Vec::new();
            map_m_ref_index(aa, |ind, entry| match entry {
                YamlValue::Hash(hh) => {
                    result.push(hh);
                    Ok(())
                },
                _ => Err(LayoutError::EntryNotHash(key.to_owned(), ind))
            })?;
            Ok(result)
        },
        Err(LayoutError::MissingKey(_)) => Ok(Vec::new()),
        _ =>  Err(LayoutError::KeyNotArray(key.to_owned())),
    }
}

fn ensure_array_of_string<'a, 'b>(
    value: Result<&'a YamlValue, LayoutError>, key: &'a str
) -> Result<Vec<String>, LayoutError> {
    match value {
        Ok(YamlValue::Array(aa)) => {
            let mut result: Vec<String> = Vec::new();
            map_m_ref_index(aa, |ind, entry| match entry {
                YamlValue::String(hh) => {
                    result.push(hh.to_owned());
                    Ok(())
                },
                _ => Err(LayoutError::EntryNotString(key.to_owned(), ind))
            })?;
            Ok(result)
        },
        Err(LayoutError::MissingKey(_)) => Ok(Vec::new()),
        _ =>  Err(LayoutError::KeyNotArray(key.to_owned())),
    }
}

pub fn lookup_yaml<'a, 'b>(key: &'a str, mapping: &'a YamlMap) -> Result<&'a YamlValue, LayoutError> {
    let key_as_yaml = YamlValue::String(key.to_owned());
    match mapping.get(&key_as_yaml) {
        None => Err(LayoutError::MissingKey(key.to_owned())),
        Some(value) => Ok(value),
    }
}

fn lookup_yaml_str<'a, 'b>(key: &'a str, mapping: &'a YamlMap) -> Result<&'a str, LayoutError> {
    let key_as_yaml = YamlValue::String(key.to_owned());
    match mapping.get(&key_as_yaml) {
        None => Err(LayoutError::MissingKey(key.to_owned())),
        Some(value) => match value {
            YamlValue::String(ss) => Ok(ss),
            _ => Err(LayoutError::MissingKey(key.to_owned())),
        }
    }
}

fn lookup_yaml_hash<'a, 'b>(key: &'a str, mapping: &'a YamlMap) -> Result<&'a YamlMap, LayoutError> {
    let key_as_yaml = YamlValue::String(key.to_owned());
    match mapping.get(&key_as_yaml) {
        None => Err(LayoutError::MissingKey(key.to_owned())),
        Some(value) => match value {
            YamlValue::Hash(hh) => Ok(hh),
            _ => Err(LayoutError::MissingKey(key.to_owned())),
        }
    }
}

fn lookup_yaml_array<'a, 'b>(key: &'a str, mapping: &'a YamlMap) -> Result<&'a Vec<YamlValue>, LayoutError> {
    let key_as_yaml = YamlValue::String(key.to_owned());
    match mapping.get(&key_as_yaml) {
        None => Err(LayoutError::MissingKey(key.to_owned())),
        Some(value) => match value {
            YamlValue::Array(aa) => Ok(aa),
            _ => Err(LayoutError::MissingKey(key.to_owned())),
        }
    }
}
