#[derive(Debug, PartialEq, Eq)]
pub enum TemplateElement {
    PlainText(String),
    Replace { identifier: String, pipe: Option<String> }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Pipe {
    name: String,
    params: Vec<String>
}
