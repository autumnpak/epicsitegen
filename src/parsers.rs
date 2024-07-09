use crate::template::{
  TemplateElement, TemplateValue, TemplateValueAccess, ForGrouping, ForSortAndFilter
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
    Rule::lookup_catcher => parse_catcher(&mut pair.into_inner()),
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
        value_pipe: Vec::new(),
        contents_pipe: parse_pipes(&mut pair.next().unwrap().into_inner())
      }
    },
    Rule::file_at_with_pipes => {
      let mut inner = filename.into_inner();
      TemplateElement::FileAt{
        snippet,
        value: parse_value(inner.next().unwrap()),
        value_pipe: parse_pipes(&mut inner.next().unwrap().into_inner()),
        contents_pipe: parse_pipes(&mut pair.next().unwrap().into_inner()),
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
        Rule::index => {
          let inner = ii.into_inner().next().unwrap();
          match inner.as_rule() {
            Rule::numbers => TemplateValueAccess::Index(inner.as_str().parse::<usize>().unwrap()),
            Rule::value => TemplateValueAccess::IndexAt(parse_value(inner)),
            _ => unreachable!("parse file element")
          }
        } 
        _ => unreachable!(),
      }).collect();
      TemplateValue{ base: name, accesses }
    }
    _ => unreachable!("parse value but was {}", pair),
  }
}

fn parse_pipes(pairs: &mut Pairs<Rule>) -> Vec<Pipe> {
  fn parse_named_pipe(pairs: &mut Pairs<Rule>) -> Pipe {
    let name = pairs.next().unwrap().as_str().to_owned();
    let params = pairs.map(|ii| ii.as_str()[1..ii.as_str().len()].to_owned()).collect();
    Pipe::Named{name, params}
  }
  pairs.map(|xx| match xx.as_rule() {
    Rule::pipe_named => parse_named_pipe(&mut xx.into_inner()),
    Rule::pipe_template => Pipe::Template,
    _ => unreachable!("pipe types"),
  }).collect()
}

fn parse_filenames(pairs: &mut Pairs<Rule>) -> Vec<String> {
  pairs.map(|ii| ii.as_str().to_owned()).collect()
}

fn parse_values(pairs: &mut Pairs<Rule>) -> Vec<TemplateValue> {
  pairs.map(|ii| { parse_value(ii) }).collect()
}

fn parse_for_element(pairs: &mut Pairs<Rule>) -> TemplateElement {
  let name = pairs.next().unwrap().as_str().to_string();
  let grouping = parse_for_groups(&mut pairs.next().unwrap().into_inner());
  let mut possibly_group = pairs.next().unwrap();
  let mut sort_and_filter = ForSortAndFilter {
    sort_key: None,
    filter_includes: None,
    filter_excludes: None,
    is_sort_ascending: false,
  };
  possibly_group = match possibly_group.as_rule() {
    Rule::for_sort_and_filter => {
      sort_and_filter = parse_for_sort_filter(&mut possibly_group.into_inner());
      pairs.next().unwrap()
    },
    Rule::ast => {
      possibly_group
    },
    _ => unreachable!("for loop thing"),
  };
  let main = possibly_group.into_inner().map(parse_ast_node).collect();
  let separator = match pairs.next().expect("e false").into_inner().next() {
    None => vec![],
    Some(ss) => ss.into_inner().map(parse_ast_node).collect(),
  };
  TemplateElement::For{name, groupings: vec![grouping], main: main, separator, sort_and_filter}
}

fn parse_for_sort_filter(pairs: &mut Pairs<Rule>) -> ForSortAndFilter {
  let mut sort_key = None;
  let mut filter_includes = None;
  let mut filter_excludes = None;
  let mut is_sort_ascending = false;
  let mut running = true;
  while running {
    match pairs.next() {
      Some(pp) => match pp.as_rule() {
        Rule::for_sort => {
          let mut pairs2 = pp.into_inner();
          is_sort_ascending = if pairs2.next().unwrap().as_str() == "sort-desc" {false} else {true};
          sort_key = Some(parse_value(pairs2.next().unwrap()));
        },
        Rule::for_filters => {
          let mut pairs2 = pp.into_inner();
          let mut running2 = true;
          while running2 {
            match pairs2.next() {
              Some(pp) => match pp.as_rule() {
                Rule::for_include => filter_includes = Some(parse_value(pp.into_inner().next().unwrap())),
                Rule::for_exclude => filter_excludes = Some(parse_value(pp.into_inner().next().unwrap())),
                _ => unreachable!("for sort and filter options"),
              },
              None => running2 = false
            }
          }
        },
        _ => unreachable!("for sort and filter options"),
      },
      None => { running = false; }
    };
  };
  ForSortAndFilter{ sort_key, filter_includes, filter_excludes, is_sort_ascending }
}

fn parse_for_groups(pairs: &mut Pairs<Rule>) -> ForGrouping {
  let mut values: Vec<TemplateValue> = Vec::new();
  let mut filenames: Vec<String> = Vec::new();
  let mut files_at: Vec<TemplateValue> = Vec::new();
  let mut running = true;
  while running {
    match pairs.next() {
      Some(pp) => match pp.as_rule() {
        Rule::for_in => values = parse_values(&mut pp.into_inner().next().unwrap().into_inner()),
        Rule::for_in_file => filenames = parse_filenames(&mut pp.into_inner().next().unwrap().into_inner()),
        Rule::for_in_file_at => files_at = parse_values(&mut pp.into_inner().next().unwrap().into_inner()),
        _ => unreachable!("for loop grouping options"),
      },
      None => { running = false; }
    };
  };
  ForGrouping { values, filenames, files_at }
}

fn parse_catcher(pairs: &mut Pairs<Rule>) -> TemplateElement {
  TemplateElement::LookupCatcher(pairs.map(|nn| nn.into_inner().map(parse_ast_node).collect()).collect())
}

pub fn parse_template_string(input: &str) -> Result<Vec<TemplateElement>, Error<Rule>> {
    let mut parsed = TemplateParser::parse(Rule::string_template, input)?;
    let ast = parsed.next().unwrap(); //never fails
    Ok(ast.into_inner().map(parse_ast_node).collect())
}

pub fn parse_mapping_string(input: &str) -> Result<Vec<TemplateValue>, Error<Rule>> {
    let mut parsed = TemplateParser::parse(Rule::string_mapping, input)?;
    let ast = parsed.next().unwrap(); //never fails
    Ok(ast.into_inner().map(parse_value).collect())
}
