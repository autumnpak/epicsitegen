#[derive(Debug, PartialEq, Eq)]
pub enum TemplateElement {
    PlainText(String),
    PlainTextWithOpen(String),
    Replace { identifier: String, pipe: Vec<String> }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Pipe {
    name: String,
    params: Vec<String>
}

impl TemplateElement {
    fn render(self) -> String {
        match self {
            TemplateElement::PlainText(text) => text.clone(),
            TemplateElement::PlainTextWithOpen(text) => String::from("{") + text.as_str(),
            _ => "".to_string()
        }
    }
}
