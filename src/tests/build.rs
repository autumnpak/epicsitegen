use crate::io::{ReadsFiles};
use crate::build::{BuildAction, BuildMultiplePages, BuildError};
use crate::yaml::{YamlMap};
use crate::template::{default_template_context, TemplateError};
use crate::tests::common::{TestFileCache, setup_io, setup_pipes};
use crate::utils::map_m;
use yaml_rust2::{yaml::{Hash, Yaml}, YamlLoader};

fn runs(
    action: BuildAction,
    io: &mut impl ReadsFiles,
) {
    let expanded = action.expand(&setup_pipes(), io, &default_template_context());
    match expanded {
        Err(BuildError::TemplateError(TemplateError::ParseError(ref ee))) => println!("{}", ee),
        _ => ()
    };
    let render = match map_m(expanded.unwrap(), |ii| {ii.run(&setup_pipes(), io, &default_template_context())}) {
        Err(BuildError::TemplateError(TemplateError::ParseError(ref ee))) => {
            println!("{}", ee);
            Err(BuildError::TemplateError(TemplateError::ParseError(ee.to_owned())))
        },
        Err(errr) => Err(errr),
        _ => Ok(())
    };
    assert_eq!(Ok(()), render);
}

fn params(strr: &str) -> YamlMap {
    let parsed = YamlLoader::load_from_str(strr).unwrap();
    let doc = &parsed[0];
    let pp: Hash = doc.as_hash().expect("not a hash map?").clone();
    pp
}

#[test]
fn Build_single_page() {
    let mut io = setup_io();
    runs(BuildAction::BuildPage{output: "out.txt".to_string(), input: "base01.txt".to_string(), params: params("bar: test")}, &mut io);
    assert_eq!(1, io.written.len());
    io.assert_written("build/out.txt", "foo test yay");
}

#[test]
fn Single_page_context() {
    let mut io = setup_io();
    runs(BuildAction::BuildPage{output: "blah/um/out.txt".to_string(), input: "base02.txt".to_string(), params: params("bar: test")}, &mut io);
    assert_eq!(1, io.written.len());
    io.assert_written("build/blah/um/out.txt", "base02.txt blah/um/out.txt build/ build/blah/um/out.txt ../../");
}

#[test]
fn Build_no_multiple_pages() {
    let mut io = setup_io();
    runs(BuildAction::BuildMultiplePages{default_params: params("{}"), on: vec![
        BuildMultiplePages{
            mapping: params("{}"),
            files: vec![],
            params: vec![],
        },
    ], descriptor: "uh".to_owned()}, &mut io);
    assert_eq!(0, io.written.len());
}

#[test]
fn Build_multiple_pages_not_from_files() {
    let mut io = setup_io();
    runs(BuildAction::BuildMultiplePages{default_params: params("{}"), on: vec![
        BuildMultiplePages{
            mapping: params("{}"),
            files: vec![],
            params: vec![
                params("input: base01.txt\noutput: out1.txt\nbar: test"),
                params("input: base01.txt\noutput: out2.txt\nbar: testing"),
            ],
        },
    ], descriptor: "uh".to_owned()}, &mut io);
    assert_eq!(2, io.written.len());
    io.assert_written("build/out1.txt", "foo test yay");
    io.assert_written("build/out2.txt", "foo testing yay");
}
