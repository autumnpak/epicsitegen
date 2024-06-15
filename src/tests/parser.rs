use crate::template::{render, TemplateError};
use crate::pipes::{PipeMap, PipeDefinition, new_pipe_map};
use crate::parsers::{parse_template_string};
use crate::io::{ReadsFiles, FileError};
use crate::yaml::{load_yaml, YamlValue, YamlFileError};
use yaml_rust2::{yaml::{Hash, Yaml}, YamlLoader};
use std::collections::HashMap;
use crate::tests::common::{TestFileCache, setup_io, setup_pipes};

fn accept(
    input: &str,
    params: &str,
    expected: &str)
{
    let parsed = YamlLoader::load_from_str(params).unwrap();
    let doc = &parsed[0];
    let pp: &Hash = doc.as_hash().expect("not a hash map?");
    let render = render(input, &pp, &setup_pipes(), &mut setup_io());
    match render {
        Err(TemplateError::ParseError(ref ee)) => println!("{}", ee),
        _ => ()
    }
    assert_eq!(Ok(expected.to_owned()), render);
}

fn reject(
    input: &str,
    params: &str,
    expected: TemplateError)
{
    let parsed = YamlLoader::load_from_str(params).unwrap();
    let doc = &parsed[0];
    let pp: &Hash = doc.as_hash().expect("not a hash map?");
    let render = render(input, &pp, &setup_pipes(), &mut setup_io());
    match render {
        Err(TemplateError::ParseError(ref ee)) => println!("{}", ee),
        _ => ()
    }
    assert_eq!(Err(expected), render);
}

#[test]
fn Empty_text() {
    accept("", "{}", "");
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
fn Basic_replacement_no_text_after() {
    accept("foo {{bar}}", "bar: test", "foo test");
}

#[test]
fn Basic_replacement_text_after() {
    accept("foo {{bar}} yay", "bar: test", "foo test yay");
}

#[test]
fn Basic_replacement_integer() {
    accept("foo {{bar}} yay", "bar: 123", "foo 123 yay");
}

#[test]
fn Basic_replacement_real() {
    accept("foo {{bar}} yay", "bar: 123.456", "foo 123.456 yay");
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
fn Replacement_with_bad_field_access() {
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

#[test]
fn if_exists_true() {
    accept("foo {% if-exists filename %}bar{% endif %} yay", "filename: bbb.txt", "foo bar yay");
}

#[test]
fn if_exists_true_else() {
    accept("foo {% if-exists filename %}bar{% else %}something{% endif %} yay", "filename: bbb.txt", "foo bar yay");
}

#[test]
fn if_exists_false() {
    accept("foo {% if-exists erm %}bar{% endif %} yay", "filename: bbb.txt", "foo  yay");
}

#[test]
fn if_exists_false_else() {
    accept("foo {% if-exists erm %}bar{% else %}something{% endif %} yay", "filename: bbb.txt", "foo something yay");
}

#[test]
fn for_loop_basic() {
    accept("foo {% for it in numbers %}{{it}} {% endfor %}yay", "numbers: [2, 4, 6]", "foo 2 4 6 yay");
}

#[test]
fn for_loop_values() {
    accept("foo {% for it in numbers, morenumbers %}{{it}} {% endfor %}yay", "numbers: [2, 4, 6]\nmorenumbers: [1, 3]", "foo 2 4 6 1 3 yay");
}

#[test]
fn for_loop_over_file() {
    accept("foo {% for it in-file entry1.yaml %}{{it}} {% endfor %}yay", "numbers: [2, 4, 6]\nmorenumbers: [1, 3]", "foo 9 8 yay");
}

#[test]
fn for_loop_over_file_at() {
    accept("foo {% for it in-file-at loc %}{{it}} {% endfor %}yay", "loc: entry2.yaml\nnumbers: [2, 4, 6]\nmorenumbers: [1, 3]", "foo asd fgh yay");
}

#[test]
fn for_loop_over_everything() {
    accept("foo {% for it in numbers, morenumbers in-file entry1.yaml in-file-at loc %}{{it}} {% endfor %}yay", "loc: entry2.yaml\nnumbers: [2, 4, 6]\nmorenumbers: [1, 3]", "foo 2 4 6 1 3 9 8 asd fgh yay");
}

#[test]
fn for_loop_over_everything_with_new_lines() {
    accept("foo {% for it\n  in numbers, morenumbers\n  in-file entry1.yaml\n  in-file-at loc %}{{it}} {% endfor %}yay", "loc: entry2.yaml\nnumbers: [2, 4, 6]\nmorenumbers: [1, 3]", "foo 2 4 6 1 3 9 8 asd fgh yay");
}

#[test]
fn replacement_with_template_pipe_1() {
    accept("foo {{bar | test0}} yay", "bar: test", "foo um1 yay");
}

#[test]
fn replacement_with_pipe_template_2() {
    accept("foo {{bar | test1}} yay", "bar: test", "foo um2 test yay");
}

#[test]
fn replacement_with_pipe_template_3() {
    accept("foo {{bar | test2}} yay", "bar: {nah: yeah}", "foo um3 yeah yay");
}

#[test]
fn replacement_with_function_pipe_1() {
    accept("foo {{bar | testfn}} yay", "bar: {nah: yeah}", "foo bleh yay");
}
