use epicsitegen::template::render;
use serde_yaml::{from_str, Mapping};

fn accept(
    input: &str,
    params: &str, 
    expected: &str)
{
    let pp: Mapping = from_str(params).expect(&"");
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
