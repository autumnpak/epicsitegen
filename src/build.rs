use crate::yaml::{YamlMap, YamlValue, YamlFileError};
use crate::template::{TemplateError, render};
use crate::pipes::{PipeMap};
use crate::io::{ReadsFiles, FileError};
use crate::utils::{map_m};

pub enum BuildError {
    FileError(FileError),
    YamlFileError(YamlFileError),
    TemplateError(TemplateError),
    TemplateErrorForFile(String, TemplateError),
    BuildMultipleFileIsntArray(String),
    BuildMultipleFileContainsNonMap(String),
    BuildMultipleInputNotSpecified(String),
    BuildMultipleOutputNotSpecified(String),
    BuildMultipleMappingYamlError(String),
}

pub enum ParamsSource {
    File(String),
    None
}

pub struct SourcedParams(YamlMap, ParamsSource);

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
    pub mappings: YamlMap,
}

impl BuildAction {
    pub fn run(&self, pipes: &PipeMap, io: &mut impl ReadsFiles) -> Result<(), BuildError> {
        match self {
            BuildAction::BuildPage{output, input, params} => {
                build_page(&output, &input, params, pipes, io)
            },
            BuildAction::BuildMultiplePages{default_params, on} => {
                map_m(on, |xx| build_multiple_pages(default_params, xx, pipes, io))
            },
            BuildAction::CopyFiles{to, from} => {
                io.copy_files(from, to).map_err(|ee| BuildError::FileError(ee))
            },
            _ => Ok(())
        }
    }
}

fn build_multiple_pages_files(
    default_params: &YamlMap,
    on: &BuildMultiplePages,
    pipes: &PipeMap,
    io: &mut impl ReadsFiles
) -> Result<Vec<SourcedParams>, BuildError> {
    let mut entries: Vec<SourcedParams> = vec![];
    let mut fileentries: Vec<Vec<()>> = map_m(&on.files, |file| {
        let contents: YamlValue = match io.read_yaml(file) {
            Ok(aa) => Ok(aa.to_owned()),
            Err(ee) => Err(BuildError::YamlFileError(ee)),
        }?;
        let arr: Vec<YamlValue> = match contents {
            YamlValue::Array(aa) => Ok(aa),
            _ => Err(BuildError::BuildMultipleFileIsntArray(file.to_owned()))
        }?;
        map_m(&arr, |aa| match aa {
            YamlValue::Hash(hh) => Ok(entries.push(SourcedParams(hh.to_owned(), ParamsSource::File(file.to_owned())))),
            _ => Err(BuildError::BuildMultipleFileContainsNonMap(file.to_owned()))
        })
    })?;
    for param in on.params {
        entries.push(SourcedParams(param.to_owned(), ParamsSource::None))
    }
    Ok(entries)
}

fn build_page(
    output: &str,
    input: &str,
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
