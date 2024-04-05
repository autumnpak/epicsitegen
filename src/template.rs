use serde_yaml::{Value, Mapping};
use crate::yaml::{lookup_yaml_map, tostr};
use crate::parsers::parse_template_elements;

#[derive(Debug, PartialEq, Eq)]
pub enum TemplateElement {
    PlainText(String),
    PlainTextWithOpen(String),
    Replace { identifier: String, pipe: Vec<String> }
}

#[derive(Debug, PartialEq, Eq)]
pub enum TemplateError {
    KeyNotPresent(String),
    ParseError(String),
    SerialisationError(String),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Pipe {
    name: String,
    params: Vec<String>
}

impl TemplateElement {
    fn render<'a>(&'a self, params: &'a Mapping) -> Result<String, TemplateError> {
        match self {
            TemplateElement::PlainText(text) => Ok(text.clone()),
            TemplateElement::PlainTextWithOpen(text) => Ok(String::from("{") + text.as_str()),
            TemplateElement::Replace{identifier, ..} => {
                let lookup = lookup_yaml_map(params, identifier)?;
                tostr(lookup)
            },
        }
    }
}

pub fn render_elements<'a>(elements: &'a Vec<TemplateElement>, params: &'a Mapping) -> Result<String, TemplateError> {
    elements.iter().try_fold("".to_owned(), |acc, ii| {
        match ii.render(params) {
            err @ Err(..) => err,
            Ok(result) => {
                let mut string = acc.to_owned();
                string.push_str(&result);
                Ok(string)
            }
        }
    })
}

pub fn render<'a>(input: &'a str, params: &'a Mapping) -> Result<String, TemplateError> {
    match parse_template_elements(input) {
        Err(ee) => Err(TemplateError::ParseError(ee.to_string())),
        Ok((_, elements)) => render_elements(&elements, params)
    }
}
