use crate::yaml::{
    YamlValue,
    YamlMap,
    new_yaml_map,
    tostr
};
use crate::io::{ReadsFiles, ReadsFilesImpl};
use crate::template::{
    TemplateElement, TemplateError, render, render_elements, TemplateValue, TemplateContext
};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Pipe {
    Named{name: String, params: Vec<String>},
    Template
}

pub enum PipeDefinition {
    Template(Vec<TemplateElement>, u128),
    Fn(
        fn(
            &YamlValue,
            &Vec<String>,
            &PipeMap,
            &ReadsFilesImpl,
        ) -> Result<YamlValue, String>, u128
    )
}

pub enum PipeInputSource<'a> {
    Value(&'a TemplateValue),
    File(&'a str),
    FileFrom(&'a str, &'a TemplateValue),
}

impl<'a> std::fmt::Display for PipeInputSource<'a> {
    fn fmt(&self, ff: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PipeInputSource::Value(strr) => write!(ff, "\"{}\"", strr),
            PipeInputSource::File(strr) => write!(ff, "the file \"{}\"", strr),
            PipeInputSource::FileFrom(strr, val) => write!(ff, "the file \"{}\" from \"{}\"", strr, val),
        }
    }
}

pub type PipeMap = HashMap<String, PipeDefinition>;
pub fn new_pipe_map() -> PipeMap { HashMap::new() }

pub fn execute_pipes<'a>(
    value: &'a YamlValue,
    pipes: &Vec<Pipe>,
    params: &'a YamlMap,
    valuepath: PipeInputSource<'a>,
    pipemap: &'a PipeMap,
    io: &mut impl ReadsFiles,
    context: &TemplateContext,
) -> Result<YamlValue, TemplateError> {
    let mut current = value.clone();
    for (ind, ii) in pipes.iter().enumerate() {
        match ii {
            Pipe::Named{name, params} => {
                current = execute_named_pipe(
                    &current, name, params, ind, 
                    &valuepath, pipemap, io, context,
                )?;
            },
            Pipe::Template => {
                current = YamlValue::String(render(&tostr(value)?, params, pipemap, io, context)
                    .map_err(|ee| TemplateError::WithinTemplatePipe(
                            Box::new(ee), ind, valuepath.to_string()
                    ))?);
            }
        }
    }
    Ok(current)
}

pub fn execute_named_pipe<'a>(
    value: &'a YamlValue,
    pipe: &str,
    pipe_params: &Vec<String>,
    index: usize,
    valuepath: &PipeInputSource<'a>,
    pipemap: &'a PipeMap,
    io: &mut impl ReadsFiles,
    context: &TemplateContext,
) -> Result<YamlValue, TemplateError> {
    match pipemap.get(pipe) {
        Some(PipeDefinition::Template(elements, _)) => {
            let mut map = new_yaml_map();
            map.insert(YamlValue::String("params".to_owned()), YamlValue::Array(pipe_params.iter().map(
                    |ii| YamlValue::String(ii.clone())
            ).collect()));
            let params_map = match value {
                YamlValue::Hash(map) => map,
                 _ => {
                    map.insert(YamlValue::String("it".to_owned()), value.clone());
                    &map
                }
            };
            let rendered = render_elements(elements, &params_map, pipemap, io, context,)
                .map_err(|ee| TemplateError::WithinTemplateNamedPipe(Box::new(ee), pipe.to_owned(), index, valuepath.to_string()))?;
            Ok(YamlValue::String(rendered))
        },
        Some(PipeDefinition::Fn(func, _)) => {
            let ioimpl: ReadsFilesImpl = ReadsFilesImpl {
                read: &|filename| io.read(filename).map(|ii| ii.to_owned())
            };
            match func(value, pipe_params, pipemap, &ioimpl) {
                Ok(strr) => Ok(strr),
                Err(ee) => Err(TemplateError::PipeExecutionError(ee, pipe.to_owned(), index, valuepath.to_string()))
            }
        },
        None => Err(TemplateError::PipeMissing(pipe.to_owned()))
    }
}
