use epicsitegen::template::render;
use epicsitegen::io::{ReadsFiles, FileError};
use yaml_rust2::{yaml::{Hash, Yaml}, YamlLoader};
use std::collections::HashMap;

pub struct TestFileCache {
    files: HashMap<String, String>,
}

impl ReadsFiles for TestFileCache {
    fn read(&mut self, filename: &str) -> Result<&str, FileError> {
        match self.files.get(filename) {
            Some(ss) => Ok(ss),
            None => Err(FileError::FileNotFound(filename.to_owned())),
        }
    }
}

fn setup_io() -> TestFileCache {
    let mut files = HashMap::new();
    files.insert("aaa.txt".to_string(), "apple".to_string());
    files.insert("bbb.txt".to_string(), "banana".to_string());
    files.insert("ccc.txt".to_string(), "carrot".to_string());
    files.insert("resources/snippets/aaa.txt".to_string(), "sapple".to_string());
    files.insert("resources/snippets/bbb.txt".to_string(), "sbanana".to_string());
    files.insert("resources/snippets/ccc.txt".to_string(), "scarrot".to_string());
    TestFileCache{files}
}

fn accept(
    input: &str,
    params: &str,
    expected: &str)
{
    let parsed = YamlLoader::load_from_str(params).unwrap();
    let doc = &parsed[0];
    let pp: &Hash = doc.as_hash().expect("not a hash map?");
    assert_eq!(Ok(expected.to_owned()), render(input, &pp, &mut setup_io()));
}

#[test]
fn Just_plain_text() {
    accept("test test", "{}", "test test");
}
#[test]
fn Just_plain_text_with_open_brace() {
    accept("test { test", "{}", "test { test");
}
#[test]
fn Basic_replacement() {
    accept("foo {{bar}}", "bar: test", "foo test");
}
#[test]
fn Basic_replacement_2() {
    accept("foo {{bar}} yay", "bar: test", "foo test yay");
}
#[test]
fn Basic_replacement_with_spaces() {
    accept("foo {{   bar  }} yay", "bar: test", "foo test yay");
}
#[test]
fn Replacement_with_field_access() {
    accept("foo {{bar.test}} yay", "bar: \n  test: something", "foo something yay");
}
#[test]
fn Replacement_with_index_access() {
    accept("foo {{bar[1]}} yay", "bar: [a, b, c, d]", "foo b yay");
}
#[test]
fn Replacement_with_many_field_accesses() {
    accept("foo {{bar.test.something.another}} yay", "bar: \n  test:\n    something:\n      another:\n        ok", "foo ok yay");
}
#[test]
fn basic_snippet() {
    accept("foo {% snippet aaa.txt %} yay", "filename: bbb.txt", "foo sapple yay");
}
#[test]
fn basic_snippet_at() {
    accept("foo {% snippet @ filename %} yay", "filename: bbb.txt", "foo sbanana yay");
}
#[test]
fn basic_file() {
    accept("foo {% file aaa.txt %} yay", "filename: bbb.txt", "foo apple yay");
}
#[test]
fn basic_file_at() {
    accept("foo {% file @ filename %} yay", "filename: bbb.txt", "foo banana yay");
}
