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
    Template(Vec<TemplateElement>),
    Fn(
        fn(
            &YamlValue,
            &PipeMap,
            &ReadsFilesImpl,
        ) -> Result<YamlValue, String>
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
            PipeInputSource::Value(strr) => write!(ff, "{}", strr),
            PipeInputSource::File(strr) => write!(ff, "the file {}", strr),
            PipeInputSource::FileFrom(strr, val) => write!(ff, "the file {} from {}", strr, val),
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
            Pipe::Named{name, ..} => {
                current = execute_named_pipe(
                    &current, name, ind, 
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
    index: usize,
    valuepath: &PipeInputSource<'a>,
    pipemap: &'a PipeMap,
    io: &mut impl ReadsFiles,
    context: &TemplateContext,
) -> Result<YamlValue, TemplateError> {
    let mut map = new_yaml_map();
    let params_map = match value {
        YamlValue::Hash(map) => map,
         _ => {
            map.insert(YamlValue::String("it".to_owned()), value.clone());
            &map
        }
    };
    let input = YamlValue::Hash(params_map.clone());
    match pipemap.get(pipe) {
        Some(PipeDefinition::Template(elements)) => {
            let rendered = render_elements(elements, params_map, pipemap, io, context,)
                .map_err(|ee| TemplateError::WithinTemplateNamedPipe(Box::new(ee), pipe.to_owned(), index, valuepath.to_string()))?;
            Ok(YamlValue::String(rendered))
        },
        Some(PipeDefinition::Fn(func)) => {
            let ioimpl: ReadsFilesImpl = ReadsFilesImpl {
                read: &|filename| io.read(filename).map(|ii| ii.to_owned())
            };
            match func(&input, pipemap, &ioimpl) {
                Ok(strr) => Ok(strr),
                Err(ee) => Err(TemplateError::PipeExecutionError(ee, pipe.to_owned(), index, valuepath.to_string()))
            }
        },
        None => Err(TemplateError::PipeMissing(pipe.to_owned()))
    }
}
/*
pub fn pipe_success(
    func: fn(&YamlValue, &PipeMap, &ReadsFilesImpl) -> String,
) -> PipeDefinition {
    let xx = |input, pipes, io| Ok(YamlValue::String(func(input, pipes, io)));
    PipeDefinition::Fn(xx)
}

pub fn pipe_str_to_str(
    func: fn(&str, &PipeMap, &ReadsFilesImpl) -> String,
) -> PipeDefinition {
    PipeDefinition::Fn(|input, pipes, io|
        match input {
            YamlValue::String(strr) => Ok(YamlValue::String(func(strr, pipes, io))),
            _ => Err("Pipe expects a string but it got something else".to_owned())
        }
    )
}*/
