use crate::template::{
  TemplateElement, TemplateValue, TemplateValueAccess
};
use pest::{
  iterators::{Pair, Pairs},
  error::Error,
  Parser
};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct TemplateParser;

fn parse_ast_node(pair: Pair<Rule>) -> TemplateElement {
  match pair.as_rule() {
    Rule::plain_text => TemplateElement::PlainText(pair.as_str().to_string()),
    Rule::replacement => TemplateElement::Replace {
      value: parse_value(pair.into_inner().next().unwrap()), pipe: vec!() 
    },
    Rule::snippet => parse_file_element(true, pair.into_inner().next().unwrap()),
    Rule::file_element => parse_file_element(false, pair.into_inner().next().unwrap()),
    Rule::if_exists => parse_if_exists_element(&mut pair.into_inner()),
    Rule::for_element => parse_for_element(&mut pair.into_inner()),
    _ => unreachable!("parse ast node"),
  }
}

fn parse_file_element(snippet: bool, pair: Pair<Rule>) -> TemplateElement {
  match pair.as_rule() {
    Rule::filename => TemplateElement::File{
      snippet,
      filename: pair.as_str().to_string(),
      pipe: vec!()
    },
    Rule::file_at => TemplateElement::FileAt{
      snippet,
      value: parse_value(pair.into_inner().next().unwrap()),
      pipe: vec!()
    },
    _ => unreachable!("parse file element")
  }
}

fn parse_for_element(pairs: &mut Pairs<Rule>) -> TemplateElement {
  let name = pairs.next().unwrap().as_str().to_string();
  let value = parse_value(pairs.next().unwrap());
  let main = pairs.next().expect("e true").into_inner().map(parse_ast_node).collect();
  let separator = match pairs.next().expect("e false").into_inner().next() {
    None => vec![],
    Some(ss) => ss.into_inner().map(parse_ast_node).collect(),
  };
  TemplateElement::For{name, value, main, separator}
}

fn parse_if_exists_element(pairs: &mut Pairs<Rule>) -> TemplateElement {
  let test = parse_value(pairs.next().unwrap());
  let when_true = pairs.next().expect("e true").into_inner().map(parse_ast_node).collect();
  let when_false = match pairs.next().expect("e false").into_inner().next() {
    None => vec![],
    Some(ss) => ss.into_inner().map(parse_ast_node).collect(),
  };
  TemplateElement::IfExists{value: test, when_true, when_false}
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
    _ => unreachable!("parse valye"),
  }
}

pub fn parse_template_string(input: &str) -> Result<Vec<TemplateElement>, Error<Rule>> {
    let mut parsed = TemplateParser::parse(Rule::file, input)?;
    let ast = parsed.next().unwrap(); //never fails
    Ok(ast.into_inner().map(parse_ast_node).collect())
}
