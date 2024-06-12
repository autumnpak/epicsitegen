use crate::io::{ReadsFiles, FileError};
use crate::build::{BuildAction, BuildMultiplePages};
use crate::yaml::{YamlMap};
use crate::tests::common::{TestFileCache, setup_io, setup_pipes};
use yaml_rust2::{yaml::{Hash, Yaml}, YamlLoader};

fn runs(
    action: BuildAction,
    io: &mut impl ReadsFiles,
) {
    assert_eq!(Ok(()), action.run(&setup_pipes(), io));
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
    io.assert_written("out.txt", "foo test yay");
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
    ]}, &mut io);
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
    ]}, &mut io);
    assert_eq!(2, io.written.len());
    io.assert_written("out1.txt", "foo test yay");
    io.assert_written("out2.txt", "foo testing yay");
}
