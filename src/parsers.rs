use crate::template::{
  TemplateElement, TemplateValue, TemplateValueAccess
};
use pest::{
  iterators::Pair,
  error::Error,
  Parser
};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammarsimple.pest"]
struct TemplateParser;

fn parse_ast_node(pair: Pair<Rule>) -> TemplateElement {
  match pair.as_rule() {
    Rule::plain_text => TemplateElement::PlainText(pair.as_str().to_string()),
    Rule::replacement => TemplateElement::Replace {
      value: parse_value(pair.into_inner().next().unwrap()), pipe: vec!() 
    },
    _ => unreachable!(),
  }
}

fn parse_value(pair: Pair<Rule>) -> TemplateValue {
  match pair.as_rule() {
    Rule::value => {
      let mut inner = pair.into_inner();
      let name = inner.next().unwrap().as_str().to_string();
      let accesses: Vec<TemplateValueAccess> = inner.map(|ii| match ii.as_rule() {
        Rule::field => TemplateValueAccess::Field(ii.into_inner().as_str().to_string()),
        Rule::index => TemplateValueAccess::Index(ii.into_inner().as_str().parse::<usize>().unwrap()),
        _ => unreachable!(),
      }).collect();
      TemplateValue{ base: name, accesses }
    }
    _ => unreachable!(),
  }
}

pub fn parse_template_string(input: &str) -> Result<Vec<TemplateElement>, Error<Rule>> {
    let mut parsed = TemplateParser::parse(Rule::file, input)?;
    let ast = parsed.next().unwrap(); //never fails

    Ok(ast.into_inner().map(parse_ast_node).collect())
}
