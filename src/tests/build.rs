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
fn build_single_page() {
    let mut io = setup_io();
    runs(BuildAction::BuildPage{output: "out.txt".to_string(), input: "base01.txt".to_string(), params: params("bar: test")}, &mut io);
    assert_eq!(1, io.written.len());
    io.assert_written("build/out.txt", "foo test yay");
}

#[test]
fn single_page_context() {
    let mut io = setup_io();
    runs(BuildAction::BuildPage{output: "blah/um/out.txt".to_string(), input: "base02.txt".to_string(), params: params("bar: test")}, &mut io);
    assert_eq!(1, io.written.len());
    io.assert_written("build/blah/um/out.txt", "base02.txt blah/um/out.txt build/ build/blah/um/out.txt ../../");
}

#[test]
fn build_single_cache_file_is_newest() {
    let mut io = setup_io();
    runs(BuildAction::BuildPage{output: "out.txt".to_string(), input: "base05.txt".to_string(), params: params("bar: test")}, &mut io);
    assert_eq!(2, io.written.len());
    io.assert_written("cache/aaa.txt__ch1", "ch1 apple");
    io.assert_written("build/out.txt", "foo ch1 apple yay");
}

#[test]
fn build_single_cache_cache_is_newest() {
    let mut io = setup_io();
    runs(BuildAction::BuildPage{output: "out.txt".to_string(), input: "base06.txt".to_string(), params: params("bar: test")}, &mut io);
    assert_eq!(1, io.written.len());
    io.assert_written("build/out.txt", "foo ch2 apple but cached yay");
    io.assert_read("cache/aaa.txt__ch2");
}

#[test]
fn build_single_cache_pipe_is_newest() {
    let mut io = setup_io();
    runs(BuildAction::BuildPage{output: "out.txt".to_string(), input: "base07.txt".to_string(), params: params("bar: test")}, &mut io);
    assert_eq!(2, io.written.len());
    io.assert_written("cache/aaa.txt__ch3", "ch3 apple");
    io.assert_written("build/out.txt", "foo ch3 apple yay");
}

#[test]
fn build_single_cache_doesnt_exist() {
    let mut io = setup_io();
    runs(BuildAction::BuildPage{output: "out.txt".to_string(), input: "base08.txt".to_string(), params: params("bar: test")}, &mut io);
    assert_eq!(2, io.written.len());
    io.assert_written("cache/aaa.txt__ch4", "ch4 apple");
    io.assert_written("build/out.txt", "foo ch4 apple yay");
}

#[test]
fn build_no_multiple_pages() {
    let mut io = setup_io();
    runs(BuildAction::BuildMultiplePages{default_params: params("{}"), on: vec![
        BuildMultiplePages{
            mapping: params("{}"),
            files: vec![],
            params: vec![],
            flatten: None,
        },
    ], descriptor: "uh".to_owned(), include: None, exclude: None}, &mut io);
    assert_eq!(0, io.written.len());
}

#[test]
fn build_multiple_pages_not_from_files() {
    let mut io = setup_io();
    runs(BuildAction::BuildMultiplePages{default_params: params("{}"), on: vec![
        BuildMultiplePages{
            mapping: params("{}"),
            files: vec![],
            params: vec![
                params("input: base01.txt\noutput: out1.txt\nbar: test"),
                params("input: base01.txt\noutput: out2.txt\nbar: testing"),
            ],
            flatten: None,
        },
    ], descriptor: "uh".to_owned(), include: None, exclude: None}, &mut io);
    assert_eq!(2, io.written.len());
    io.assert_written("build/out1.txt", "foo test yay");
    io.assert_written("build/out2.txt", "foo testing yay");
}

#[test]
fn build_multiple_pages_mapping() {
    let mut io = setup_io();
    runs(BuildAction::BuildMultiplePages{default_params: params("{}"), on: vec![
        BuildMultiplePages{
            mapping: params("mapped: \"{{bar}}{{yay}}\""),
            files: vec![],
            params: vec![
                params("input: base03.txt\noutput: out1.txt\nbar: test\nyay: nah"),
                params("input: base03.txt\noutput: out2.txt\nbar: testing\nyay: nah2"),
            ],
            flatten: None,
        },
    ], descriptor: "uh".to_owned(), include: None, exclude: None}, &mut io);
    assert_eq!(2, io.written.len());
    io.assert_written("build/out1.txt", "foo testnah yay");
    io.assert_written("build/out2.txt", "foo testingnah2 yay");
}

#[test]
fn build_multiple_pages_include() {
    let mut io = setup_io();
    runs(BuildAction::BuildMultiplePages{default_params: params("{}"), on: vec![
        BuildMultiplePages{
            mapping: params("{}"),
            files: vec![],
            params: vec![
                params("input: base01.txt\noutput: out1.txt\nbar: test9\niii: whee\neee: whoo"),
                params("input: base01.txt\noutput: out2.txt\nbar: test8\neee: whoo"),
                params("input: base01.txt\noutput: out3.txt\nbar: test7\niii: whee"),
                params("input: base01.txt\noutput: out4.txt\nbar: test6\n")
            ],
            flatten: None,
        },
    ], descriptor: "uh".to_owned(), include: Some("iii".to_owned()), exclude: None}, &mut io);
    assert_eq!(2, io.written.len());
    io.assert_written("build/out1.txt", "foo test9 yay");
    io.assert_written("build/out3.txt", "foo test7 yay");
}

#[test]
fn build_multiple_pages_exclude() {
    let mut io = setup_io();
    runs(BuildAction::BuildMultiplePages{default_params: params("{}"), on: vec![
        BuildMultiplePages{
            mapping: params("{}"),
            files: vec![],
            params: vec![
                params("input: base01.txt\noutput: out1.txt\nbar: test9\niii: whee\neee: whoo"),
                params("input: base01.txt\noutput: out2.txt\nbar: test8\neee: whoo"),
                params("input: base01.txt\noutput: out3.txt\nbar: test7\niii: whee"),
                params("input: base01.txt\noutput: out4.txt\nbar: test6\n")
            ],
            flatten: None,
        },
    ], descriptor: "uh".to_owned(), exclude: Some("eee".to_owned()), include: None}, &mut io);
    assert_eq!(2, io.written.len());
    io.assert_written("build/out4.txt", "foo test6 yay");
    io.assert_written("build/out3.txt", "foo test7 yay");
}

#[test]
fn build_multiple_pages_includes_exclude() {
    let mut io = setup_io();
    runs(BuildAction::BuildMultiplePages{default_params: params("{}"), on: vec![
        BuildMultiplePages{
            mapping: params("{}"),
            files: vec![],
            params: vec![
                params("input: base01.txt\noutput: out1.txt\nbar: test9\niii: whee\neee: whoo"),
                params("input: base01.txt\noutput: out2.txt\nbar: test8\neee: whoo"),
                params("input: base01.txt\noutput: out3.txt\nbar: test7\niii: whee"),
                params("input: base01.txt\noutput: out4.txt\nbar: test6\n")
            ],
            flatten: None,
        },
    ], descriptor: "uh".to_owned(), exclude: Some("eee".to_owned()), include: Some("iii".to_owned())}, &mut io);
    assert_eq!(1, io.written.len());
    io.assert_written("build/out3.txt", "foo test7 yay");
}

#[test]
fn build_multiple_pages_flatten() {
    let mut io = setup_io();
    runs(BuildAction::BuildMultiplePages{default_params: params("{}"), on: vec![
        BuildMultiplePages{
            mapping: params("{output: \"out{{flat}}.txt\"}"),
            files: vec![],
            params: vec![
                params("input: base04.txt\nbar: test\nflat: [99, 88, 77]"),
            ],
            flatten: Some("flat".to_owned()),
        },
    ], descriptor: "uh".to_owned(), include: None, exclude: None}, &mut io);
    assert_eq!(3, io.written.len());
    io.assert_written("build/out99.txt", "0 99\n---\n- 99\n- 88\n- 77");
    io.assert_written("build/out88.txt", "1 88\n---\n- 99\n- 88\n- 77");
    io.assert_written("build/out77.txt", "2 77\n---\n- 99\n- 88\n- 77");
}
