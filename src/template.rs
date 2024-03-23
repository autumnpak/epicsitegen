#[derive(Debug)]
enum TemplateElement {
    PlainText(String),
    Replace { identifier: String, pipe: Option<String> }
}
