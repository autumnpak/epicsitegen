use crate::yaml::{
    lookup_value,
    tostr,
    YamlMap,
    YamlValue,
    to_iterable,
    insert_value,
    YamlFileError,
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
        values: Vec<TemplateValue>,
        filenames: Vec<String>,
        files_at: Vec<TemplateValue>,
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
    YamlFileError(YamlFileError),
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
            TemplateElement::For{name, values, filenames, files_at, main, separator, ..} => {
                let over = for_make_iterable(params, values, filenames, files_at, io)?;
                let mut mapped: Vec<String> = map_m(over, |ii| {
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

fn for_make_iterable(
    params: & YamlMap,
    values: &Vec<TemplateValue>,
    filenames: &Vec<String>,
    files_at: &Vec<TemplateValue>,
    io: &mut impl ReadsFiles
) -> Result<Vec<YamlValue>, TemplateError> {
    let mut entries = Vec::new();
    for value in values {
        let lookup = lookup_value(&value, params)?;
        let mut as_vec = to_iterable(lookup)?;
        entries.append(&mut as_vec);
    }
    for filename in filenames {
        let lookup = io.read_yaml(filename)
            .map_err(|xx| TemplateError::YamlFileError(xx))?;
        let mut as_vec = to_iterable(lookup)?;
        entries.append(&mut as_vec);
    }
    for fileat in files_at {
        let lookup = lookup_value(&fileat, params)?;
        let filename = tostr(lookup)?;
        let file = io.read_yaml(&filename)
            .map_err(|xx| TemplateError::YamlFileError(xx))?;
        let mut as_vec = to_iterable(file)?;
        entries.append(&mut as_vec);
    }
    Ok(entries)
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
