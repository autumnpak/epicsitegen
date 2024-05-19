use crate::template::{
  TemplateElement, TemplateValue, TemplateValueAccess
};
use crate::pipes::{
  Pipe
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
    Rule::replacement => {
      let mut iter = pair.into_inner();
      TemplateElement::Replace {
        value: parse_value(iter.next().unwrap()),
        pipe: parse_pipes(&mut iter.next().unwrap().into_inner()) 
      }
    } ,
    Rule::snippet => parse_file_element(true, &mut pair.into_inner()),
    Rule::file_element => parse_file_element(false, &mut pair.into_inner()),
    Rule::if_exists => parse_if_exists_element(&mut pair.into_inner()),
    Rule::for_element => parse_for_element(&mut pair.into_inner()),
    _ => unreachable!("parse ast node"),
  }
}

fn parse_file_element(snippet: bool, pair: &mut Pairs<Rule>) -> TemplateElement {
  let filename = pair.next().unwrap();
  match filename.as_rule() {
    Rule::filename => {
      TemplateElement::File{
        snippet,
        filename: filename.as_str().to_string(),
        pipe: parse_pipes(&mut pair.next().unwrap().into_inner())
      }
    },
    Rule::file_at => {
      TemplateElement::FileAt{
        snippet,
        value: parse_value(filename.into_inner().next().unwrap()),
        pipe: parse_pipes(&mut pair.next().unwrap().into_inner())
      }
    },
    _ => unreachable!("parse file element")
  }
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
    _ => unreachable!("parse value but was {}", pair),
  }
}

fn parse_pipes(pairs: &mut Pairs<Rule>) -> Vec<Pipe> {
  fn parse_pipe(pairs: &mut Pairs<Rule>) -> Pipe {
    let name = pairs.next().unwrap().as_str().to_owned();
    let params = pairs.map(|ii| ii.as_str().to_owned()).collect();
    Pipe{name, params}
  }
  pairs.map(|xx| parse_pipe(&mut xx.into_inner())).collect()
}

fn parse_filenames(pairs: &mut Pairs<Rule>) -> Vec<String> {
  pairs.map(|ii| ii.as_str().to_owned()).collect()
}

fn parse_values(pairs: &mut Pairs<Rule>) -> Vec<TemplateValue> {
  pairs.map(|ii| { println!("|{}|", ii.as_str()); parse_value(ii) }).collect()
}

fn parse_for_element(pairs: &mut Pairs<Rule>) -> TemplateElement {
  let name = pairs.next().unwrap().as_str().to_string();
  let mut values: Vec<TemplateValue> = Vec::new();
  let mut filenames: Vec<String> = Vec::new();
  let mut files_at: Vec<TemplateValue> = Vec::new();
  let mut main: Option<Vec<TemplateElement>> = Option::None;
  while main.is_none() {
    let next = pairs.next().unwrap();
    match next.as_rule() {
      Rule::for_in => values = parse_values(&mut next.into_inner().next().unwrap().into_inner()),
      Rule::for_in_file => filenames = parse_filenames(&mut next.into_inner().next().unwrap().into_inner()),
      Rule::for_in_file_at => files_at = parse_values(&mut next.into_inner().next().unwrap().into_inner()),
      Rule::ast => main = Some(next.into_inner().map(parse_ast_node).collect()),
      _ => unreachable!("for loop options"),
    };
  };
  let separator = match pairs.next().expect("e false").into_inner().next() {
    None => vec![],
    Some(ss) => ss.into_inner().map(parse_ast_node).collect(),
  };
  TemplateElement::For{name, values, filenames, files_at, main: main.unwrap(), separator}
}

pub fn parse_template_string(input: &str) -> Result<Vec<TemplateElement>, Error<Rule>> {
    let mut parsed = TemplateParser::parse(Rule::file, input)?;
    let ast = parsed.next().unwrap(); //never fails
    Ok(ast.into_inner().map(parse_ast_node).collect())
}
