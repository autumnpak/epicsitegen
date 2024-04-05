use serde_yaml::{Value, Mapping};
use crate::yaml::{lookup_yaml_map, tostr};

#[derive(Debug, PartialEq, Eq)]
pub enum TemplateElement {
    PlainText(String),
    PlainTextWithOpen(String),
    Replace { identifier: String, pipe: Vec<String> }
}

pub enum TemplateError<'a> {
    KeyNotPresent(&'a str),
    SerialisationError(String)
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

pub fn render<'a>(elements: Vec<&'a TemplateElement>, params: &'a Mapping) -> Result<String, TemplateError<'a>> {
    elements.iter().try_fold("".to_owned(), |acc, &ii| {
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
