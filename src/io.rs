use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::path::Path;
use std::fs;
use std::fmt;
use crate::yaml::{YamlValue, YamlFileError, load_yaml};

#[derive(Debug, PartialEq, Eq)]
pub enum FileError {
    FileNotFound(String),
    FileCantBeRead(String),
}

pub trait ReadsFiles {
    fn read(&mut self, filename: &str) -> Result<&str, FileError>;
    fn read_yaml(&mut self, filename: &str) -> Result<&YamlValue, YamlFileError>;
}

//thing we need because we can't use 'impl ReadsFiles' in PipeDefinition's type definition
pub struct ReadsFilesImpl<'a> {
    pub read: &'a dyn FnMut(&'a str) -> Result<String, FileError>
}
impl<'a> fmt::Display for ReadsFilesImpl<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "reads files impl",)
    }
}

pub struct FileCache {
    files: HashMap<String, String>,
    yamls: HashMap<String, YamlValue>,
    file_pipes: HashMap<(String, Vec<String>), String>,
}

fn read_file(filename: &str) -> Result<String, FileError> {
    if Path::exists(Path::new(filename)) {
        match fs::read_to_string(filename) {
            Ok(strr) => Ok(strr),
            Err(_) => Err(FileError::FileCantBeRead(filename.to_owned())),
        }
    } else {
        Err(FileError::FileNotFound(filename.to_owned()))
    }
}

impl ReadsFiles for FileCache {
    fn read(&mut self, filename: &str) -> Result<&str, FileError> {
        Ok(match self.files.entry(filename.to_owned()) {
            Entry::Occupied(ee) => ee.into_mut(),
            Entry::Vacant(ee) => ee.insert(read_file(filename)?),
        })
    }

    fn read_yaml(&mut self, filename: &str) -> Result<&YamlValue, YamlFileError> {
        let contentsref = self.read(filename).map_err(|xx| YamlFileError::File(xx))?;
        let contents = contentsref.to_owned();
        Ok(match self.yamls.entry(filename.to_owned()) {
            Entry::Occupied(ee) => ee.into_mut(),
            Entry::Vacant(ee) => {
                ee.insert(load_yaml(&contents).map_err(|xx| YamlFileError::Yaml(xx))?)
            }
        })
    }
}
