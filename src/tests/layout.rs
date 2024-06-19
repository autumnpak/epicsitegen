use crate::yaml::{YamlValue, YamlMap, load_yaml};
use crate::build::{BuildAction, BuildMultiplePages};
use crate::layout::{layout_string_to_buildactions};
use yaml_rust2::{yaml::{Hash, Yaml}, YamlLoader};

fn params(strr: &str) -> YamlMap {
    let parsed = YamlLoader::load_from_str(strr).unwrap();
    let doc = &parsed[0];
    let pp: Hash = doc.as_hash().expect("not a hash map?").clone();
    pp
}

#[test]
fn parses_copy_ok() {
    let parsed = layout_string_to_buildactions("- type: copy\n  from: place\n  to: elsewhere");
    assert_eq!(Ok(vec![BuildAction::CopyFiles{
        from: "place".to_owned(),
        to: "elsewhere".to_owned(),
    }]), parsed);
}

#[test]
fn parses_build_ok() {
    let parsed = layout_string_to_buildactions("- type: build\n  input: place\n  output: elsewhere\n  params: {um: yeah}");
    assert_eq!(Ok(vec![BuildAction::BuildPage{
        input: "place".to_owned(),
        output: "elsewhere".to_owned(),
        params: params("{um: yeah}"),
    }]), parsed);
}

#[test]
fn parses_build_multiple_ok() {
    let parsed = layout_string_to_buildactions("- type: build-multiple
  description: uh
  default: {um: yeah}
  with:
    - files: [yeah]
      params: [{um2: yeah2}]
      mapping: {foo: asd}");
    assert_eq!(Ok(vec![BuildAction::BuildMultiplePages{default_params: params("{um: yeah}"), on: vec![
        BuildMultiplePages{
            mapping: params("{foo: asd}"),
            files: vec!["yeah".to_owned()],
            params: vec![params("{um2: yeah2}")],
        },
    ], descriptor: "uh".to_owned()}]), parsed);
}

#[test]
fn build_multiple_allows_no_files() {
    let parsed = layout_string_to_buildactions("- type: build-multiple
  description: uh
  default: {um: yeah}
  with:
    - params: [{um2: yeah2}]
      mapping: {foo: asd}");
    assert_eq!(Ok(vec![BuildAction::BuildMultiplePages{default_params: params("{um: yeah}"), on: vec![
        BuildMultiplePages{
            mapping: params("{foo: asd}"),
            files: vec![],
            params: vec![params("{um2: yeah2}")],
        },
    ], descriptor: "uh".to_owned()}]), parsed);
}

#[test]
fn build_multiple_allows_no_params() {
    let parsed = layout_string_to_buildactions("- type: build-multiple
  description: uh
  default: {um: yeah}
  with:
    - files: [yeah]
      mapping: {foo: asd}");
    assert_eq!(Ok(vec![BuildAction::BuildMultiplePages{default_params: params("{um: yeah}"), on: vec![
        BuildMultiplePages{
            mapping: params("{foo: asd}"),
            files: vec!["yeah".to_owned()],
            params: vec![],
        },
    ], descriptor: "uh".to_owned()}]), parsed);
}

#[test]
fn build_multiple_allows_no_mapping() {
    let parsed = layout_string_to_buildactions("- type: build-multiple
  description: uh
  default: {um: yeah}
  with:
    - files: [yeah]
      params: [{um2: yeah2}]");
    assert_eq!(Ok(vec![BuildAction::BuildMultiplePages{default_params: params("{um: yeah}"), on: vec![
        BuildMultiplePages{
            mapping: params("{}"),
            files: vec!["yeah".to_owned()],
            params: vec![params("{um2: yeah2}")],
        },
    ], descriptor: "uh".to_owned()}]), parsed);
}
