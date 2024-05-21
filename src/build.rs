use crate::yaml::{YamlMap, YamlFileError};
use crate::template::{TemplateError, render};
use crate::pipes::{PipeMap};
use crate::io::{ReadsFiles, FileError};

pub enum BuildError {
    FileError(FileError),
    TemplateError(TemplateError),
}

pub enum BuildAction {
    BuildPage {output: String, input: String, params: YamlMap},
    BuildMultiplePages {
        input: String,
        default_params: YamlMap,
        filename: String,
    },
    CopyFiles {to: String, from: String},
}

pub struct BuildMultiplePages {
    pub files: Vec<String>,
    pub mappings: YamlMap,
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

impl BuildAction {
    pub fn run(&self, pipes: &PipeMap, io: &mut impl ReadsFiles) -> Result<(), BuildError> {
        match self {
            BuildAction::BuildPage{output, input, params} => {
                build_page(&output, &input, params, pipes, io)
            }
            BuildAction::CopyFiles{to, from} => {
                io.copy_files(from, to).map_err(|ee| BuildError::FileError(ee))
            }
            _ => Ok(())
        }
    }
}
