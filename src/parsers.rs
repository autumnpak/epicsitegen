use crate::template::{TemplateElement};
use pest::{
  iterators::Pair,
  error::Error,
  Parser
};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammarsimple.pest"]
struct TemplateParser;

pub fn parse_template_string(file: &str) -> Result<Vec<TemplateElement>, Error<Rule>> {
    let mut parsed = TemplateParser::parse(Rule::file, file)?;
    let ast = parsed.next().unwrap(); //never fails

    fn parse_value(pair: Pair<Rule>) -> TemplateElement {
      match pair.as_rule() {
        Rule::plain_text => TemplateElement::PlainText(pair.as_str().to_string()),
        Rule::replacement => TemplateElement::Replace { 
          identifier: pair.into_inner().as_str().to_string(), pipe: vec!() 
        },
        _ => unreachable!(),
      }
    }
    Ok(ast.into_inner().map(parse_value).collect())
}
