use crate::yaml::{YamlMap, YamlValue, YamlFileError};
use crate::template::{TemplateError, render, render_elements};
use crate::pipes::{PipeMap};
use crate::io::{ReadsFiles, FileError};
use crate::parsers::{parse_template_string};
use crate::utils::{map_m, map_m_ref, fold_m_mut, map_m_mut};

#[derive(Debug, PartialEq, Eq)]
pub enum BuildError {
    Sourced(Box<BuildError>),
    FileError(FileError),
    YamlFileError(YamlFileError),
    TemplateError(TemplateError),
    TemplateErrorForFile(String, TemplateError),
    BMFIsntArray(String),
    BMFContainsNonMap(String),
    BMInputNotSpecified(String),
    BMOutputNotSpecified(String),
    BMMappingParseError(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParamsSource {
    File(String),
    None
}

#[derive(Debug, PartialEq, Eq)]
pub struct SourcedParams(YamlMap, ParamsSource, YamlMap); //params, source, mapping

#[derive(Debug, PartialEq, Eq)]
pub struct SourcedParamsWithFiles(YamlMap, ParamsSource, String, String);

pub enum BuildAction {
    BuildPage {output: String, input: String, params: YamlMap},
    BuildMultiplePages {
        default_params: YamlMap,
        on: Vec<BuildMultiplePages>,
    },
    CopyFiles {to: String, from: String},
}

pub struct BuildMultiplePages {
    pub files: Vec<String>,
    pub params: Vec<YamlMap>,
    pub mapping: YamlMap,
}

impl BuildAction {
    pub fn run(&self, pipes: &PipeMap, io: &mut impl ReadsFiles) -> Result<(), BuildError> {
        match self {
            BuildAction::BuildPage{output, input, params} => {
                build_page(&input, &output, params, pipes, io)
            },
            BuildAction::BuildMultiplePages{default_params, on} => {
                let mut entries: Vec<SourcedParams> = vec![];
                for mut source in map_m_ref(on, |xx| build_multiple_pages_files(xx, pipes, io))? {
                    entries.append(&mut source);
                };
                let mapped = build_multiple_pages_map_params(default_params, entries, pipes, io)?;
                build_multiple_pages_actually_build(mapped, pipes, io)
            },
            BuildAction::CopyFiles{to, from} => {
                io.copy_files(from, to).map_err(|ee| BuildError::FileError(ee))
            },
            _ => Ok(())
        }
    }
}

fn build_multiple_pages_files(
    on: &BuildMultiplePages,
    pipes: &PipeMap,
    io: &mut impl ReadsFiles
) -> Result<Vec<SourcedParams>, BuildError> {
    let mut entries: Vec<SourcedParams> = vec![];
    map_m_ref(&on.files, |file| {
        let contents: YamlValue = match io.read_yaml(&file) {
            Ok(aa) => Ok(aa.to_owned()),
            Err(ee) => Err(BuildError::YamlFileError(ee)),
        }?;
        let arr: Vec<YamlValue> = match contents {
            YamlValue::Array(aa) => Ok(aa),
            _ => Err(BuildError::BMFIsntArray(file.to_owned()))
        }?;
        map_m(arr, |aa| match aa {
            YamlValue::Hash(hh) => Ok(entries.push(
                    SourcedParams(hh.to_owned(), ParamsSource::File(file.to_owned()), on.mapping.to_owned())
            )),
            _ => Err(BuildError::BMFContainsNonMap(file.to_owned()))
        })
    })?;
    for param in &on.params {
        entries.push(SourcedParams(param.to_owned(), ParamsSource::None, on.mapping.to_owned()))
    }
    Ok(entries)
}

fn build_multiple_pages_map_params(
    default_params: &YamlMap,
    values: Vec<SourcedParams>,
    pipes: &PipeMap,
    io: &mut impl ReadsFiles
) -> Result<Vec<SourcedParamsWithFiles>, BuildError> {
    map_m_mut(values, |ii| {
        let mut params: YamlMap = default_params.to_owned();
        params.extend(ii.0);
        Ok(SourcedParamsWithFiles(params, ii.1, "".to_owned(), "".to_owned()))
    })
}

pub fn apply_mapping<'a>(
    dest: &'a mut YamlMap,
    mapping: &'a mut YamlMap,
    pipes: &PipeMap,
    io: &mut impl ReadsFiles
) -> Result<(), BuildError> {
    for (key, value) in mapping {
        match value {
            YamlValue::String(ss) => {
                match parse_template_string(ss) {
                    Err(ee) => return Err(BuildError::BMMappingParseError(ee.to_string())),
                    Ok(elements) => {
                        let elements = render_elements(&elements, dest, pipes, io)
                            .map_err(|xx| BuildError::TemplateError(xx))?;
                        dest.insert(key.to_owned(), YamlValue::String(elements));
                    }
                }
            },
            _ => ()
        }
    }
    Ok(())
}

fn build_multiple_pages_actually_build(
    values: Vec<SourcedParamsWithFiles>,
    pipes: &PipeMap,
    io: &mut impl ReadsFiles
) -> Result<(), BuildError> {
    fold_m_mut((), values, |_, ii: SourcedParamsWithFiles| {
        build_page(&ii.2, &ii.3, &ii.0, pipes, io)
            .map_err(|ee| BuildError::Sourced(Box::new(ee)))
    })?;
    Ok(())
}

fn build_page(
    input: &str,
    output: &str,
    params: &YamlMap,
    pipes: &PipeMap,
    io: &mut impl ReadsFiles
) -> Result<(), BuildError> {
    let contents = match io.read(input) {
        Ok(ss) => Ok(ss.to_owned()),
        Err(ee) => Err(BuildError::FileError(ee)),
    }?;
    let rendered = render(&contents, params, pipes, io)
        .map_err(|xx| BuildError::TemplateError(xx))?;
    io.write(output, &rendered).map_err(|xx| BuildError::FileError(xx))
}
