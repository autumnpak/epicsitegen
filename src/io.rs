use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::path::Path;
use std::fs;

#[derive(Debug, PartialEq, Eq)]
pub enum FileError {
    FileNotFound(String),
    FileCantBeRead(String),
}

pub trait ReadsFiles {
    fn read(&mut self, filename: &str) -> Result<&str, FileError>;
}

pub struct FileCache {
    files: HashMap<String, String>,
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
}
