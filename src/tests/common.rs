use crate::pipes::{PipeMap, PipeDefinition, new_pipe_map};
use crate::parsers::{parse_template_string};
use crate::template::{TemplateElement};
use crate::io::{ReadsFiles, FileError};
use crate::yaml::{load_yaml, YamlValue, YamlFileError};
use yaml_rust2::{yaml::{Yaml},};
use std::collections::{HashMap, HashSet};

pub struct TestFileCache {
    files: HashMap<String, String>,
    file_modify_times: HashMap<String, u128>,
    yamls: HashMap<String, YamlValue>,
    templates: HashMap<String, Vec<TemplateElement>>,
    pub read: HashSet<String>,
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

    pub fn assert_read(&self, filename: &str) -> &str {
        println!("{:?}", self.read);
        self.read.get(filename).expect(&format!("{} was not read from", filename))
    }
}

impl ReadsFiles for TestFileCache {
    fn read(&mut self, filename: &str) -> Result<&str, FileError> {
        self.read.insert(filename.to_owned());
        match self.files.get(filename) {
            Some(ss) => Ok(ss),
            None => Err(FileError::FileNotFound(filename.to_owned())),
        }
    }

    fn modify_time(&self, filename: &str) -> Option<u128> {
        self.file_modify_times.get(filename).map(|ii| *ii)
    }

    fn read_yaml(&mut self, filename: &str) -> Result<&YamlValue, YamlFileError> {
        self.read.insert(filename.to_owned());
        let contents = self.read(filename).map_err(|xx| YamlFileError::File(xx))?;
        let loaded = load_yaml(contents).map_err(|xx| YamlFileError::Yaml(xx))?;
        self.yamls.insert(filename.to_owned(), loaded);
        Ok(self.yamls.get(filename).unwrap())
    }

    fn read_template(&mut self, filename: &str) -> Result<&Vec<TemplateElement>, FileError> {
        self.read.insert(filename.to_owned());
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
    let mut file_modify_times = HashMap::new();
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
    files.insert("base05.txt".to_string(), "foo {%file aaa.txt | ch1%} yay".to_string());
    files.insert("base06.txt".to_string(), "foo {%file aaa.txt | ch2%} yay".to_string());
    files.insert("base07.txt".to_string(), "foo {%file aaa.txt | ch3%} yay".to_string());
    files.insert("base08.txt".to_string(), "foo {%file aaa.txt | ch4%} yay".to_string());
    files.insert("flatten_object_unused.txt".to_string(), "{{_flatten_index}} {{flat.value}}\n{{_flatten_array}}".to_string());
    files.insert("cache/aaa.txt__ch1".to_string(), "ch1 apple but cached".to_string());
    files.insert("cache/aaa.txt__ch2".to_string(), "ch2 apple but cached".to_string());
    files.insert("cache/aaa.txt__ch3".to_string(), "ch3 apple but cached".to_string());
    file_modify_times.insert("aaa.txt".to_string(), 3000);
    file_modify_times.insert("cache/aaa.txt__ch1".to_string(), 2000);
    file_modify_times.insert("cache/aaa.txt__ch2".to_string(), 4000);
    file_modify_times.insert("cache/aaa.txt__ch2".to_string(), 3000);
    TestFileCache{files, file_modify_times, read: HashSet::new(), yamls: HashMap::new(), templates: HashMap::new(), written: HashMap::new()}
}

pub fn setup_pipes() -> PipeMap {
    let mut pipemap = new_pipe_map();
    pipemap.insert("test0".to_string(), PipeDefinition::Template(parse_template_string("um1").unwrap(), None));
    pipemap.insert("test1".to_string(), PipeDefinition::Template(parse_template_string("um2 {{it}}").unwrap(), None));
    pipemap.insert("test2".to_string(), PipeDefinition::Template(parse_template_string("um3 {{nah}}").unwrap(), None));
    pipemap.insert("test3".to_string(), PipeDefinition::Template(parse_template_string("um4 {{it}} {{params[0]}} {{params[1]}}").unwrap(), None));
    pipemap.insert("ch1".to_string(), PipeDefinition::Template(parse_template_string("ch1 {{it}}").unwrap(), Some(2000)));
    pipemap.insert("ch2".to_string(), PipeDefinition::Template(parse_template_string("ch2 {{it}}").unwrap(), Some(3000)));
    pipemap.insert("ch3".to_string(), PipeDefinition::Template(parse_template_string("ch3 {{it}}").unwrap(), Some(4000)));
    pipemap.insert("ch4".to_string(), PipeDefinition::Template(parse_template_string("ch4 {{it}}").unwrap(), Some(4000)));
    pipemap.insert("txt".to_string(), PipeDefinition::Template(parse_template_string("{{it}}.txt").unwrap(), None));
    pipemap.insert("testfn".to_string(), 
        PipeDefinition::Fn(|_input, _params, _pipes, _io| Ok(Yaml::String("bleh".to_owned())), None)
    );
    pipemap.insert("testfn2".to_string(), 
        PipeDefinition::Fn(|_input, _params, _pipes, _io| Ok(Yaml::Array(vec![Yaml::Integer(1), Yaml::Integer(2)])), None)
    );
    pipemap
}


