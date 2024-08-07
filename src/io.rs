use crate::yaml::{YamlValue, YamlFileError, load_yaml};
use crate::template::{TemplateElement};
use crate::parsers::{parse_template_string};
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::path::{Path, PathBuf};
use std::fs;
use std::fmt;
use std::io;
use std::sync::{Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, PartialEq, Eq)]
pub enum FileError {
    FileNotFound(String),
    FileCantBeRead(String),
    FileCantBeWritten(String),
    FilesCantBeCopied(String),
    CantCopyDirIntoFile(String, String),
    TemplateFileFailedParsing(String, String),
}

impl std::fmt::Display for FileError {
    fn fmt(&self, ff: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileError::FileNotFound(strr) => write!(ff, "Can't find the file {}", strr),
            FileError::FileCantBeRead(strr) => write!(ff, "Can't read the file {}", strr),
            FileError::FileCantBeWritten(strr) => write!(ff, "Can't write to the file {}", strr),
            FileError::FilesCantBeCopied(strr) => write!(ff, "Can't copy the files at {}", strr),
            FileError::CantCopyDirIntoFile(from, to) => write!(ff, "Can't copy {} into {} as its a file", from, to),
            FileError::TemplateFileFailedParsing(file, error) => write!(ff, "Couldn't parse {} into templating:\n{}", file, error),
        }
    }
}

pub trait ReadsFiles {
    fn read(&mut self, filename: &str) -> Result<&str, FileError>;
    fn modify_time(&self, filename: &str) -> Option<u128>;
    fn write(&mut self, filename: &str, contents: &str) -> Result<(), FileError>;
    fn read_yaml(&mut self, filename: &str) -> Result<&YamlValue, YamlFileError>;
    fn read_template(&mut self, filename: &str) -> Result<&Vec<TemplateElement>, FileError>;
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

pub struct ThreadsafeFileCache {
    fc: FileCache,
    mutex: Mutex<()>,
}

impl ReadsFiles for ThreadsafeFileCache {
    /* There's a few assumptions I'm making here on the thread safety of this:
     * - Files to be read will NOT be updated during the build process
     * - Whenever a file is read, it's quickly transformed into some other form (YAML
     *   representation, something piped, etc)
     * - So if those files are updated anyway, there won't be a dangling reference to them because
     *   the transformation happens so quickly
     *
     * I know this is pretty naive but at the very least it will (hopefully) only be inefficient instead
     * of wrong
     */
    fn read(&mut self, filename: &str) -> Result<&str, FileError> {
        let lock = self.mutex.lock().unwrap();
        let res = self.fc.read(filename);
        drop(lock);
        res
    }

    fn modify_time(&self, filename: &str) -> Option<u128> {
        let lock = self.mutex.lock().unwrap();
        let res = self.fc.modify_time(filename);
        drop(lock);
        res
    }

    fn write(&mut self, filename: &str, contents: &str) -> Result<(), FileError> {
        let lock = self.mutex.lock().unwrap();
        let res = self.fc.write(filename, contents);
        drop(lock);
        res
    }

    fn read_yaml(&mut self, filename: &str) -> Result<&YamlValue, YamlFileError> {
        let lock = self.mutex.lock().unwrap();
        let res = self.fc.read_yaml(filename);
        drop(lock);
        res
    }

    fn read_template(&mut self, filename: &str) -> Result<&Vec<TemplateElement>, FileError> {
        let lock = self.mutex.lock().unwrap();
        let res = self.fc.read_template(filename);
        drop(lock);
        res
    }

    fn copy_files(&self, to: &str, from: &str) -> Result<(), FileError> {
        let lock = self.mutex.lock().unwrap();
        let res = self.fc.copy_files(to, from);
        drop(lock);
        res
    }
}

pub struct FileCache {
    files: HashMap<String, (u128, String)>,
    yamls: HashMap<String, (u128, YamlValue)>,
    templates: HashMap<String, (u128, Vec<TemplateElement>)>,
}

impl FileCache {
    pub fn new() -> FileCache {
        FileCache {
            files: HashMap::new(),
            yamls: HashMap::new(),
            templates: HashMap::new(),
        }
    }
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

pub fn get_real_file_modify_time(filename: &str) -> Option<u128> {
    let metadata = fs::metadata(filename).ok()?;
    let modify = metadata.modified().ok()?;
    let duration = modify.duration_since(UNIX_EPOCH).ok()?;
    Some(duration.as_millis())
}

impl ReadsFiles for FileCache {
    fn read(&mut self, filename: &str) -> Result<&str, FileError> {
        let got = match self.files.entry(filename.to_owned()) {
            Entry::Occupied(mut ee) => {
                let filetime = fs::metadata(filename).unwrap().modified().unwrap()
                    .duration_since(UNIX_EPOCH).unwrap().as_millis();
                let entry: &(u128, String) = ee.get();
                if filetime >= entry.0 {
                    ee.insert((
                        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
                        read_file(filename)?
                    ));
                }
                ee.into_mut()
            },
            Entry::Vacant(ee) => {
                ee.insert((
                    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
                    read_file(filename)?
                ))
            },
        };
        Ok(&got.1)
    }

    fn modify_time(&self, filename: &str) -> Option<u128> {
        get_real_file_modify_time(filename)
    }

    fn read_yaml(&mut self, filename: &str) -> Result<&YamlValue, YamlFileError> {
        let contentsref = self.read(filename).map_err(|xx| YamlFileError::File(xx))?;
        let contents = contentsref.to_owned();
        let got = match self.yamls.entry(filename.to_owned()) {
            Entry::Occupied(mut ee) => {
                let filetime = fs::metadata(filename).unwrap().modified().unwrap()
                    .duration_since(UNIX_EPOCH).unwrap().as_millis();
                let entry: &(u128, YamlValue) = ee.get();
                if filetime >= entry.0 {
                    ee.insert((
                        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
                        load_yaml(&contents).map_err(|xx| YamlFileError::Yaml(xx))?
                    ));
                }
                ee.into_mut()
            },
            Entry::Vacant(ee) => {
                ee.insert((
                    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
                    load_yaml(&contents).map_err(|xx| YamlFileError::Yaml(xx))?
                ))
            },
        };
        Ok(&got.1)
    }

    fn read_template(&mut self, filename: &str) -> Result<&Vec<TemplateElement>, FileError> {
        let contentsref = self.read(filename)?;
        let contents = contentsref.to_owned();
        let got = match self.templates.entry(filename.to_owned()) {
            Entry::Occupied(mut ee) => {
                let filetime = fs::metadata(filename).unwrap().modified().unwrap()
                    .duration_since(UNIX_EPOCH).unwrap().as_millis();
                let entry: &(u128, Vec<TemplateElement>) = ee.get();
                if filetime >= entry.0 {
                    ee.insert((
                        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
                        parse_template_string(&contents).map_err(
                            |xx| FileError::TemplateFileFailedParsing(filename.to_owned(), xx.to_string())
                        )?
                    ));
                }
                ee.into_mut()
            },
            Entry::Vacant(ee) => {
                ee.insert((
                    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
                    parse_template_string(&contents).map_err(
                        |xx| FileError::TemplateFileFailedParsing(filename.to_owned(), xx.to_string())
                    )?
                ))
            },
        };
        Ok(&got.1)
    }

    fn write(&mut self, filename: &str, contents: &str) -> Result<(), FileError> {
        match Path::new(filename).parent() {
            Some(parent) => {
                if !parent.is_dir() {
                    fs::create_dir_all(parent);
                }
            }
            _ => {}
        }
        fs::write(filename, contents).map_err(|_xx| FileError::FileCantBeWritten(filename.to_owned()))
    }

    fn copy_files(&self, from: &str, to: &str) -> Result<(), FileError> {
        let from_path = PathBuf::from(from);
        let to_path = PathBuf::from(to);
        if from_path.is_file() {
            match fs::copy(from, to) {
                Ok(_) => Ok(()),
                Err(_ee) => Err(FileError::FilesCantBeCopied(from.to_owned())),
            }
        } else {
            if to_path.is_file() {
                Err(FileError::CantCopyDirIntoFile(from.to_owned(), to.to_owned()))
            } else {
                copy_dir_all(from, to).map_err(|_xx| FileError::FilesCantBeCopied(from.to_owned()))
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
