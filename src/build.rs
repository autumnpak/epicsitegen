use crate::yaml::{YamlMap, YamlFileError};
use crate::template::{TemplateError, render};
use crate::pipes::{PipeMap};
use crate::io::{ReadsFiles, FileError};

pub enum BuildError {
    FileError(FileError),
    TemplateError(TemplateError),
}

pub enum BuildAction {
    BuildPage {output: String, base_file: String, params: YamlMap},
    BuildMultiplePages {
        base_file: String,
        default_params: YamlMap,
        filename: String,
    },
    CopyFiles {to: String, from: String},
}

pub struct BuildMultiplePages {
    pub files: Vec<String>,
    pub mappings: YamlMap,
}

impl BuildAction {
    pub fn run(&self, pipes: &PipeMap, io: &mut impl ReadsFiles) -> Result<(), BuildError> {
        match self {
            BuildAction::BuildPage{output, base_file, params} => {
                let contents = match io.read(base_file) {
                    Ok(ss) => Ok(ss.to_owned()),
                    Err(ee) => Err(BuildError::FileError(ee)),
                }?;
                let rendered = render(&contents, params, pipes, io)
                    .map_err(|xx| BuildError::TemplateError(xx))?;
                io.write(output, &rendered).map_err(|xx| BuildError::FileError(xx))
            }
            _ => Ok(())
        }
    }
}
