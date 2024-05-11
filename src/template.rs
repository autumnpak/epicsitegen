use crate::yaml::{lookup_value, tostr, YamlMap};
use crate::parsers::parse_template_string;
use crate::io::{ReadsFiles, FileError};

#[derive(Debug, PartialEq, Eq)]
pub enum TemplateElement {
    PlainText(String),
    Replace { value: TemplateValue, pipe: Vec<String> },
    File { snippet: bool, filename: String, pipe: Vec<String> },
    FileAt { snippet: bool, value: TemplateValue, pipe: Vec<String> },
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
}

#[derive(Debug, PartialEq, Eq)]
pub struct Pipe {
    pub name: String,
    pub params: Vec<String>
}

impl TemplateElement {
    fn render<'a>(&'a self, params: &'a YamlMap, io: &mut impl ReadsFiles) -> Result<String, TemplateError> {
        match self {
            TemplateElement::PlainText(text) => Ok(text.clone()),
            TemplateElement::Replace{value, ..} => {
                let lookup = lookup_value(value, params)?;
                tostr(lookup)
            },
            TemplateElement::File{snippet, filename, ..} => {
                let realFilename = format!("{}{}", if *snippet {"resources/snippets/"} else {""}, filename);
                match io.read(&realFilename) {
                    Ok(strr) => Ok(strr.to_owned()),
                    Err(ee) => Err(TemplateError::FileError(ee))
                }
            },
            TemplateElement::FileAt{snippet, value, ..} => {
                let lookup = lookup_value(value, params)?;
                let filename = tostr(lookup)?;
                let realFilename = format!("{}{}", if *snippet {"resources/snippets/"} else {""}, filename);
                match io.read(&realFilename) {
                    Ok(strr) => Ok(strr.to_owned()),
                    Err(ee) => Err(TemplateError::FileError(ee))
                }
            }
        }
    }
}

pub fn render_elements<'a>(elements: &'a Vec<TemplateElement>, params: &'a YamlMap, io: &mut impl ReadsFiles) -> Result<String, TemplateError> {
    elements.iter().try_fold("".to_owned(), |acc, ii| {
        match ii.render(params, io) {
            err @ Err(..) => err,
            Ok(result) => {
                let mut string = acc.to_owned();
                string.push_str(&result);
                Ok(string)
            }
        }
    })
}

pub fn render<'a>(input: &'a str, params: &'a YamlMap, io: &mut impl ReadsFiles) -> Result<String, TemplateError> {
    match parse_template_string(input) {
        Err(ee) => Err(TemplateError::ParseError(ee.to_string())),
        Ok(elements) => render_elements(&elements, params, io)
    }
}
