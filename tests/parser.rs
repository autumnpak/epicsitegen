use epicsitegen::template::render;
use yaml_rust2::{yaml::{Hash, Yaml}, YamlLoader};

fn accept(
    input: &str,
    params: &str,
    expected: &str)
{
    let parsed = YamlLoader::load_from_str(params).unwrap();
    let doc = &parsed[0];
    match doc {
        Yaml::Hash(_) => println!("hash"),
        Yaml::Array(_) => println!("array"),
        Yaml::Real(_) => println!("real"),
        Yaml::Integer(_) => println!("integer"),
        Yaml::String(_) => println!("str"),
        Yaml::Boolean(_) => println!("boolean"),
        Yaml::Alias(_) => println!("alias"),
        Yaml::Null => println!("null"),
        Yaml::BadValue => println!("badvalue"),
    }
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
