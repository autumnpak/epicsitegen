use crate::pipes::{PipeMap, PipeDefinition, new_pipe_map};
use crate::parsers::{parse_template_string};
use crate::template::{TemplateElement};
use crate::io::{ReadsFiles, FileError};
use crate::yaml::{load_yaml, YamlValue, YamlFileError};
use yaml_rust2::{yaml::{Yaml},};
use std::collections::HashMap;

pub struct TestFileCache {
    files: HashMap<String, String>,
    yamls: HashMap<String, YamlValue>,
    templates: HashMap<String, Vec<TemplateElement>>,
    pub written: HashMap<String, String>,
}

impl TestFileCache {
    pub fn assert_written(&self, filename: &str, contents: &str) {
        println!("{:?}", self.written);
        assert_eq!(
            self.written.get(filename)
                .expect(&format!("{} was not written to", filename)),
            contents
        );
    }
}

impl ReadsFiles for TestFileCache {
    fn read(&mut self, filename: &str) -> Result<&str, FileError> {
        match self.files.get(filename) {
            Some(ss) => Ok(ss),
            None => Err(FileError::FileNotFound(filename.to_owned())),
        }
    }

    fn read_yaml(&mut self, filename: &str) -> Result<&YamlValue, YamlFileError> {
        let contents = self.read(filename).map_err(|xx| YamlFileError::File(xx))?;
        let loaded = load_yaml(contents).map_err(|xx| YamlFileError::Yaml(xx))?;
        self.yamls.insert(filename.to_owned(), loaded);
        Ok(self.yamls.get(filename).unwrap())
    }

    fn read_template(&mut self, filename: &str) -> Result<&Vec<TemplateElement>, FileError> {
        let contents = self.read(filename)?;
        let loaded = parse_template_string(contents).map_err(
            |xx| FileError::TemplateFileFailedParsing(filename.to_owned(), xx.to_string())
        )?;
        self.templates.insert(filename.to_owned(), loaded);
        Ok(self.templates.get(filename).unwrap())
    }

    fn write(&mut self, filename: &str, contents: &str) -> Result<(), FileError> {
        self.written.insert(filename.to_owned(), contents.to_owned());
        Ok(())
    }

    fn copy_files(&self, _from: &str, _to: &str) -> Result<(), FileError> {
        Ok(())
    }
}

pub fn setup_io() -> TestFileCache {
    let mut files = HashMap::new();
    files.insert("aaa.txt".to_string(), "apple".to_string());
    files.insert("bbb.txt".to_string(), "banana".to_string());
    files.insert("ccc.txt".to_string(), "carrot".to_string());
    files.insert("resources/snippets/aaa.txt".to_string(), "sapple".to_string());
    files.insert("resources/snippets/bbb.txt".to_string(), "sbanana".to_string());
    files.insert("resources/snippets/ccc.txt".to_string(), "scarrot".to_string());
    files.insert("entry1.yaml".to_string(), "[9, 8]".to_string());
    files.insert("entry2.yaml".to_string(), "[\"asd\", \"fgh\"]".to_string());
    files.insert("base01.txt".to_string(), "foo {{bar}} yay".to_string());
    files.insert("base02.txt".to_string(), "{{_input}} {{_output}} {{_outputfolder}} {{_outputfull}} {{_dots}}".to_string());
    files.insert("base03.txt".to_string(), "foo {{mapped}} yay".to_string());
    files.insert("base04.txt".to_string(), "{{_flatten_index}} {{flat}}\n{{_flatten_array}}".to_string());
    TestFileCache{files, yamls: HashMap::new(), templates: HashMap::new(), written: HashMap::new()}
}

pub fn setup_pipes() -> PipeMap {
    let mut pipemap = new_pipe_map();
    pipemap.insert("test0".to_string(), PipeDefinition::Template(parse_template_string("um1").unwrap(), 0));
    pipemap.insert("test1".to_string(), PipeDefinition::Template(parse_template_string("um2 {{it}}").unwrap(), 0));
    pipemap.insert("test2".to_string(), PipeDefinition::Template(parse_template_string("um3 {{nah}}").unwrap(), 0));
    pipemap.insert("test3".to_string(), PipeDefinition::Template(parse_template_string("um4 {{it}} {{params[0]}} {{params[1]}}").unwrap(), 0));
    pipemap.insert("txt".to_string(), PipeDefinition::Template(parse_template_string("{{it}}.txt").unwrap(), 0));
    pipemap.insert("testfn".to_string(), PipeDefinition::Fn(|_input, _params, _pipes, _io| Ok(Yaml::String("bleh".to_owned())), 0));
    pipemap
}


