use crate::yaml::{
    YamlValue,
    new_yaml_map,
};
use crate::io::{ReadsFiles, ReadsFilesImpl};
use crate::template::{
    TemplateElement, TemplateError, render_elements
};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq)]
pub struct Pipe {
    pub name: String,
    pub params: Vec<String>
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

pub type PipeMap = HashMap<String, PipeDefinition>;
pub fn new_pipe_map() -> PipeMap { HashMap::new() }

pub fn execute_pipe<'a>(
    value: &'a YamlValue,
    pipe: &str,
    pipemap: &'a PipeMap,
    io: &mut impl ReadsFiles
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
            let rendered = render_elements(elements, params_map, pipemap, io)?;
            Ok(YamlValue::String(rendered))
        },
        Some(PipeDefinition::Fn(func)) => {
            let ioimpl: ReadsFilesImpl = ReadsFilesImpl {
                read: &|filename| io.read(filename).map(|ii| ii.to_owned())
            };
            match func(&input, pipemap, &ioimpl) {
                Ok(strr) => Ok(strr),
                Err(ee) => Err(TemplateError::PipeExecutionError(ee))
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
