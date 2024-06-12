use crate::yaml::{YamlValue, YamlFileError, load_yaml};
use crate::utils::{map_m};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::path::{Path, PathBuf};
use std::fs;
use std::fmt;
use std::io;
use glob::{glob, Paths, GlobError};

#[derive(Debug, PartialEq, Eq)]
pub enum FileError {
    FileNotFound(String),
    FileCantBeRead(String),
    FileCantBeWritten(String),
    FilesCantBeCopied(String),
    CantCopyDirIntoFile(String, String),
}

impl std::fmt::Display for FileError {
    fn fmt(&self, ff: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileError::FileNotFound(strr) => write!(ff, "Can't find the file {}", strr),
            FileError::FileCantBeRead(strr) => write!(ff, "Can't read the file {}", strr),
            FileError::FileCantBeWritten(strr) => write!(ff, "Can't write to the file {}", strr),
            FileError::FilesCantBeCopied(strr) => write!(ff, "Can't copy the files at {}", strr),
            FileError::CantCopyDirIntoFile(from, to) => write!(ff, "Can't copy {} into {} as its a file", from, to),
        }
    }
}

pub trait ReadsFiles {
    fn read(&mut self, filename: &str) -> Result<&str, FileError>;
    fn write(&mut self, filename: &str, contents: &str) -> Result<(), FileError>;
    fn read_yaml(&mut self, filename: &str) -> Result<&YamlValue, YamlFileError>;
    fn copy_files(&self, to: &str, from: &str) -> Result<(), FileError>;
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

    fn write(&mut self, filename: &str, contents: &str) -> Result<(), FileError> {
        fs::write(filename, contents).map_err(|xx| FileError::FileCantBeWritten(filename.to_owned()))
    }

    fn copy_files(&self, from: &str, to: &str) -> Result<(), FileError> {
        let from_path = PathBuf::from(from);
        let to_path = PathBuf::from(to);
        if from_path.is_file() {
            match fs::copy(from, to) {
                Ok(_) => Ok(()),
                Err(ee) => Err(FileError::FilesCantBeCopied(from.to_owned())),
            }
        } else {
            if to_path.is_file() {
                Err(FileError::CantCopyDirIntoFile(from.to_owned(), to.to_owned()))
            } else {
                copy_dir_all(from, to).map_err(|xx| FileError::FilesCantBeCopied(from.to_owned()))
            }
        }
    }
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}
