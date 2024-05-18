use crate::yaml::{
    lookup_value,
    tostr,
    YamlMap,
    to_iterable,
    insert_value,
};
use crate::parsers::parse_template_string;
use crate::io::{ReadsFiles, FileError};
use crate::utils::{map_m};
use crate::pipes::{
    Pipe, PipeMap, execute_pipe
};

#[derive(Debug, PartialEq, Eq)]
pub enum TemplateElement {
    PlainText(String),
    Replace { value: TemplateValue, pipe: Vec<Pipe> },
    File { snippet: bool, filename: String, pipe: Vec<Pipe> },
    FileAt { snippet: bool, value: TemplateValue, pipe: Vec<Pipe> },
    IfExists {
        value: TemplateValue,
        when_true: Vec<TemplateElement>,
        when_false: Vec<TemplateElement>
    },
    For {
        name: String,
        value: TemplateValue,
        main: Vec<TemplateElement>,
        separator: Vec<TemplateElement>
    },
}

#[derive(Debug, PartialEq, Eq)]
pub struct TemplateValue {
    pub base: String,
    pub accesses: Vec<TemplateValueAccess>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TemplateValueAccess {
    Field(String),
    Index(usize)
}

#[derive(Debug, PartialEq, Eq)]
pub enum TemplateError {
    KeyNotPresent(String),
    ParseError(String),
    SerialisationError(String),
    IndexOOB(String, usize),
    FieldNotPresent(String, String),
    IndexOnUnindexable(String, usize),
    FieldOnUnfieldable(String, String),
    FileError(FileError),
    ForOnUnindexable(String),
    PipeMissing(String),
    PipeExecutionError(String),
}
impl TemplateElement {
    fn render<'a>(&'a self, params: &'a YamlMap, pipes: &'a PipeMap, io: &mut impl ReadsFiles) -> Result<String, TemplateError> {
        match self {
            TemplateElement::PlainText(text) => Ok(text.clone()),
            TemplateElement::Replace{value, pipe} => {
                let lookup = lookup_value(value, params)?;
                let mut current = lookup.clone();
                for ii in pipe {
                    current = execute_pipe(&current, &ii.name, pipes, io)?;
                }
                tostr(&current)
            },
            TemplateElement::File{snippet, filename, pipe} => {
                let real_filename = format!("{}{}", if *snippet {"resources/snippets/"} else {""}, filename);
                match io.read(&real_filename) {
                    Ok(strr) => Ok(strr.to_owned()),
                    Err(ee) => Err(TemplateError::FileError(ee))
                }
            },
            TemplateElement::FileAt{snippet, value, pipe} => {
                let lookup = lookup_value(value, params)?;
                let filename = tostr(lookup)?;
                let real_filename = format!("{}{}", if *snippet {"resources/snippets/"} else {""}, filename);
                match io.read(&real_filename) {
                    Ok(strr) => Ok(strr.to_owned()),
                    Err(ee) => Err(TemplateError::FileError(ee))
                }
            }
            TemplateElement::IfExists{value, when_true, when_false} => {
                let lookup = lookup_value(value, params);
                match lookup {
                    Ok(..) => render_elements(when_true, params, pipes, io),
                    Err(ee) => match ee {
                        TemplateError::KeyNotPresent(..) |
                        TemplateError::FieldNotPresent(..) |
                        TemplateError::IndexOOB(..) => render_elements(when_false, params, pipes, io),
                        _ => Err(ee)
                    }
                }
            }
            TemplateElement::For{name, value, main, separator} => {
                let lookup = lookup_value(value, params)?;
                let as_vec = to_iterable(lookup)?;
                let mapped: Vec<String> = map_m(&as_vec, |ii| {
                    let mut new_params = params.clone();
                    insert_value(&mut new_params, &name, ii.clone());
                    render_elements(main, &new_params, pipes, io)
                })?;
                let sep = render_elements(separator, params, pipes, io)?;
                Ok(mapped.join(&sep))
            }
        }
    }
}

pub fn render_elements<'a>(
    elements: &'a Vec<TemplateElement>,
    params: &'a YamlMap,
    pipes: &'a PipeMap,
    io: &mut impl ReadsFiles
) -> Result<String, TemplateError> {
    elements.iter().try_fold("".to_owned(), |acc, ii| {
        match ii.render(params, pipes, io) {
            err @ Err(..) => err,
            Ok(result) => {
                let mut string = acc.to_owned();
                string.push_str(&result);
                Ok(string)
            }
        }
    })
}

pub fn render<'a>(
    input: &'a str,
    params: &'a YamlMap,
    pipes: &'a PipeMap,
    io: &mut impl ReadsFiles
) -> Result<String, TemplateError> {
    match parse_template_string(input) {
        Err(ee) => Err(TemplateError::ParseError(ee.to_string())),
        Ok(elements) => render_elements(&elements, params, pipes, io)
    }
}
