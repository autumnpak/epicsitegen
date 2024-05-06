use epicsitegen::template::render;
use yaml_rust2::{yaml::{Hash, Yaml}, YamlLoader};

fn accept(
    input: &str,
    params: &str,
    expected: &str)
{
    let parsed = YamlLoader::load_from_str(params).unwrap();
    let doc = &parsed[0];
    let pp: &Hash = doc.as_hash().expect("not a hash map?");
    assert_eq!(Ok(expected.to_owned()), render(input, &pp));
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
