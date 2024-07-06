use crate::yaml::{YamlMap, YamlValue, YamlFileError, lookup_str_from_yaml_map, lookup_yaml};
use crate::template::{TemplateError, render, render_elements, TemplateContext};
use crate::pipes::{PipeMap};
use crate::io::{ReadsFiles, FileError};
use crate::parsers::{parse_template_string};
use crate::utils::{map_m_mut, map_m_ref, map_m_index, map_m_ref_index};
use std::path::PathBuf;
use pathdiff::diff_paths;

#[derive(Debug, PartialEq, Eq)]
pub enum BuildError {
    BMSourced(ParamsSource, Box<BuildError>),
    FileError(FileError),
    YamlFileError(YamlFileError),
    TemplateError(TemplateError),
    BMFIsntArray(String),
    BMFContainsNonMap(String, usize),
    BMInputNotSpecified(String, ParamsSource),
    BMOutputNotSpecified(ParamsSource),
    BMMappingParseError(String, String, ParamsSource),
    BMMappingTemplateError(TemplateError, String, ParamsSource),
    BMMappingIsntString(String, ParamsSource),
    FlattenOnNonArray(String, ParamsSource),
    FlattenKeyNotFound(String, ParamsSource),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, ff: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::FileError(ee) => ee.fmt(ff),
            BuildError::YamlFileError(ee) => ee.fmt(ff),
            BuildError::TemplateError(ee) => ee.fmt(ff),
            BuildError::BMFIsntArray(filename) => write!(ff, "Can't build multiple files using \"{}\" as params as it doesn't contain a yaml array", filename),
            BuildError::BMFContainsNonMap(filename, idx) => write!(ff, "Can't build multiple files using \"{}\" as params because entry {} isn't a map", filename, idx),
            BuildError::BMOutputNotSpecified(source) => write!(ff, "Can't build the file at {} because it has no output specified", source),
            BuildError::BMInputNotSpecified(filename, source) => write!(ff, "Can't build \"{}\" (at {}) because it has no base input specified", filename, source),
            BuildError::BMMappingParseError(err, key, source) => write!(ff, "Can't parse mapping \"{}\" (at {}): {}", key, source, err),
            BuildError::BMMappingTemplateError(err, key, source) => write!(ff, "Can't parse mapping \"{}\" (at {}): {}", key, source, err),
            BuildError::BMMappingIsntString(key, source) => write!(ff, "Mapping \"{}\" (at {}) isn't a string", key, source),
            BuildError::FlattenOnNonArray(key, source) => write!(ff, "Can't flatten with \"{}\" (on {}) as it isn't an array", key, source),
            BuildError::FlattenKeyNotFound(key, source) => write!(ff, "Can't flatten with \"{}\" (on {}) as the value to flatten with wasn't found", key, source),
            BuildError::BMSourced(source, err) => write!(ff, "{}\nat {}", err, source),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ParamsSource{
    pub grouping: usize, 
    pub index: usize, 
    pub flatten_index: Option<usize>,
    pub file: Option<String>, 
}

impl std::fmt::Display for ParamsSource {
    fn fmt(&self, ff: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.file {
            Some(ss) => write!(ff, "{}", ss),
            None => write!(ff, "(specified params)"),
        }?;
        write!(ff, ":{}", self.index)?;
        match &self.flatten_index {
            Some(ss) => write!(ff, " (flatten result {})", ss),
            None => write!(ff, ""),
        }?;
        write!(ff, " in grouping {}", self.grouping)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SourcedParams {
    pub params: YamlMap,
    pub source: ParamsSource,
    pub mapping: YamlMap,
    pub flatten: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildAction {
    BuildPage {output: String, input: String, params: YamlMap},
    BuildMultiplePages {
        descriptor: String,
        default_params: YamlMap,
        on: Vec<BuildMultiplePages>,
        include: Option<String>,
        exclude: Option<String>,
    },
    CopyFiles {to: String, from: String},
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildActionExpanded {
    BuildPage {output: String, input: String, params: YamlMap, source: Option<ParamsSource>},
    CopyFiles {to: String, from: String},
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildMultiplePages {
    pub files: Vec<String>,
    pub params: Vec<YamlMap>,
    pub mapping: YamlMap,
    pub flatten: Option<String>,
}

impl BuildAction {
    pub fn expand(&self, pipes: &PipeMap, io: &mut impl ReadsFiles, context: &TemplateContext) -> Result<Vec<BuildActionExpanded>, BuildError> {
        match self {
            BuildAction::BuildPage{output, input, params} => {
                Ok(vec![BuildActionExpanded::BuildPage{
                    input: input.to_owned(),
                    output: output.to_owned(),
                    params: params.to_owned(),
                    source: None
                }])
            },
            BuildAction::CopyFiles{to, from} => {
                Ok(vec![BuildActionExpanded::CopyFiles{
                    to: to.to_owned(),
                    from: from.to_owned(),
                }])
            },
            BuildAction::BuildMultiplePages{default_params, on, include, exclude, ..} => {
                let mut entries: Vec<SourcedParams> = vec![];
                for mut source in map_m_ref_index(on, |idx, xx| build_multiple_pages_files(xx, idx, io))? {
                    entries.append(&mut source);
                };
                build_multiple_pages_map_params(
                    default_params, entries, &include, &exclude, 
                    pipes, io, context)
            },
        }
    }

    pub fn message_expansion_failed(&self, time: u128, error: BuildError) -> String {
        match self {
            BuildAction::BuildMultiplePages{descriptor, ..} => {
                format!("Expanding {} failed after {}ms:\n    {}", descriptor, time, str::replace(&error.to_string(), "\n", "\n    "))
            },
            _ => unreachable!() //nothing else fails to expand so this doesnt matter
        }
    }
}

impl BuildActionExpanded {
    pub fn run(&self, pipes: &PipeMap, io: &mut impl ReadsFiles, context: &TemplateContext) -> Result<(), BuildError> {
        match self {
            BuildActionExpanded::BuildPage{output, input, params, source} => {
                build_page(&input, &output, params, pipes, io, context)
                    .map_err(|ee| if let Some(ss) = source { BuildError::BMSourced(ss.to_owned(), Box::new(ee)) } else {ee} )
            },
            BuildActionExpanded::CopyFiles{to, from} => {
                io.copy_files(from, to).map_err(|ee| BuildError::FileError(ee))
            },
        }
    }

    pub fn message_run_succeeded(&self, time: u128) -> String {
        match self {
            BuildActionExpanded::BuildPage{output, ..} => {
                format!("Successfully built {} in {}ms", output, time)
            },
            BuildActionExpanded::CopyFiles{to, from} => {
                format!("Successfully copied {} to {} in {}ms", from, to, time)
            },
        }
    }

    pub fn message_run_failed(&self, time: u128, error: BuildError) -> String {
        match self {
            BuildActionExpanded::BuildPage{output, ..} => {
                format!("Building {} failed after {}ms:\n    {}", output, time, str::replace(&error.to_string(), "\n", "\n    "))
            },
            BuildActionExpanded::CopyFiles{to, from} => {
                format!("Copying {} to {} failed after {}ms:\n    {}", from, to, time, str::replace(&error.to_string(), "\n", "\n    "))
            },
        }
    }
}

fn build_multiple_pages_files(
    on: &BuildMultiplePages,
    group_index: usize,
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
        map_m_index(arr, |idx, aa| match aa {
            YamlValue::Hash(hh) => Ok(entries.push(
                    SourcedParams{
                        params: hh.to_owned(),
                        source: ParamsSource {
                            grouping: group_index, index: idx, file: Some(file.to_owned()), flatten_index: None
                        },
                        mapping: on.mapping.to_owned(),
                        flatten: on.flatten.clone(),
                    }
            )),
            _ => Err(BuildError::BMFContainsNonMap(file.to_owned(), idx))
        })
    })?;
    for (idx, param) in on.params.iter().enumerate() {
        entries.push(SourcedParams{
            params: param.to_owned(),
            source: ParamsSource{
                grouping: group_index, index: idx, file: None, flatten_index: None
            },
            mapping: on.mapping.to_owned(),
            flatten: on.flatten.clone(),
        })
    }
    Ok(entries)
}

fn build_multiple_pages_map_params(
    default_params: &YamlMap,
    values: Vec<SourcedParams>,
    include: &Option<String>,
    exclude: &Option<String>,
    pipes: &PipeMap,
    io: &mut impl ReadsFiles,
    context: &TemplateContext,
) -> Result<Vec<BuildActionExpanded>, BuildError> {
    let mut result = Vec::new();
    for ii in values {
        let mut params: YamlMap = default_params.to_owned();
        params.extend(ii.params);
        let flattened = if let Some(flat) = &ii.flatten { 
            match lookup_yaml(&flat, &params) {
                Ok(fullarray @ YamlValue::Array(aa)) => Ok(aa.iter().enumerate().map(|(ind, flatarr)| {
                    let mut newparams = params.clone();
                    newparams.insert(YamlValue::String("_flatten_array".to_owned()), fullarray.clone());
                    newparams.insert(YamlValue::String("_flatten_index".to_owned()), YamlValue::Integer(ind as i64));
                    newparams.insert(YamlValue::String(flat.to_owned()), flatarr.clone());
                    let mut new_source = ii.source.to_owned();
                    new_source.flatten_index = Some(ind);
                    (newparams, new_source)
                }).collect()),
                Ok(..) => Err(BuildError::FlattenOnNonArray(flat.to_owned(), ii.source.to_owned())),
                Err(..) => Err(BuildError::FlattenKeyNotFound(flat.to_owned(), ii.source.to_owned())),
            }
        } else { Ok(vec![(params, ii.source)]) }?;
        let mut mapped = Vec::new();
        for ff in flattened {
            mapped.push((apply_mapping(&ff.0, &ii.mapping, &ff.1, pipes, io, context)?, ff.1));
        }
        for ff in mapped {
            let source = ff.1;
            let finalparams = ff.0;
            let output = lookup_str_from_yaml_map("output", &finalparams).map_err(|_|
                BuildError::BMOutputNotSpecified(source.clone())
            )?;
            let input = lookup_str_from_yaml_map("input", &finalparams).map_err(|_|
                BuildError::BMInputNotSpecified(output.to_owned(), source.clone())
                )?;
            let included = if let Some(ss) = include {
                finalparams.contains_key(&YamlValue::String(ss.to_owned()))
            } else {true};
            let excluded = if let Some(ss) = exclude {
                !finalparams.contains_key(&YamlValue::String(ss.to_owned()))
            } else {true};
            if included && excluded {
                result.push(BuildActionExpanded::BuildPage{
                    input: input.to_owned(), output: output.to_owned(),
                    params: finalparams.to_owned(), source: Some(source.to_owned())
                });
            }
        }
    }
    Ok(result)
}

pub fn apply_mapping<'a>(
    params: &'a YamlMap,
    mapping: &'a YamlMap,
    source: &'a ParamsSource,
    pipes: &PipeMap,
    io: &mut impl ReadsFiles,
    context: &TemplateContext,
) -> Result<YamlMap, BuildError> {
    let mut mapp = params.to_owned();
    for (key, value) in mapping {
        match value {
            YamlValue::String(ss) => {
                match parse_template_string(ss) {
                    Err(ee) => return Err(BuildError::BMMappingParseError(
                            ee.to_string(),
                            key.to_owned().as_str().unwrap().to_owned(),
                            source.clone()
                        )),
                    Ok(elements) => {
                        let elements = render_elements(&elements, &mapp, pipes, io, context)
                            .map_err(|xx| BuildError::BMMappingTemplateError(
                                    xx,
                                    key.to_owned().as_str().unwrap().to_owned(), 
                                    source.clone()
                            ))?;
                        mapp.insert(key.to_owned(), YamlValue::String(elements));
                    }
                }
            },
            _ => return Err(BuildError::BMMappingIsntString(
                key.to_owned().as_str().unwrap().to_owned(),
                source.clone()
            ))
        }
    }
    Ok(mapp)
}

fn build_page(
    input: &str,
    output: &str,
    params: &YamlMap,
    pipes: &PipeMap,
    io: &mut impl ReadsFiles,
    context: &TemplateContext,
) -> Result<(), BuildError> {
    let contents = match io.read(input) {
        Ok(ss) => Ok(ss.to_owned()),
        Err(ee) => Err(BuildError::FileError(ee)),
    }?;
    let mut full_params = params.to_owned();
    full_params.insert(YamlValue::String("_input".to_owned()), YamlValue::String(input.to_owned()));
    full_params.insert(YamlValue::String("_output".to_owned()), YamlValue::String(output.to_owned()));
    full_params.insert(YamlValue::String("_outputfolder".to_owned()), YamlValue::String(context.output_folder.to_owned()));
    let fullpathbuf: PathBuf = [&context.output_folder, output].iter().collect();
    let fullpath = String::from(fullpathbuf.to_string_lossy());
    full_params.insert(YamlValue::String("_outputfull".to_owned()), YamlValue::String(fullpath.clone()));
    let mut outpath: PathBuf = output.into();
    outpath.pop();
    let basepath: PathBuf = ".".into();
    let dots = diff_paths(basepath, outpath).unwrap();
    let dotstring = dots.to_string_lossy().replace("\\", "/");
    full_params.insert(YamlValue::String("_dots".to_owned()), YamlValue::String(String::from(&dotstring[0..dotstring.len() - 1])));
    let rendered = render(&contents, &full_params, pipes, io, context)
        .map_err(|xx| BuildError::TemplateError(xx))?;
    io.write(&fullpath, &rendered).map_err(|xx| BuildError::FileError(xx))
}
