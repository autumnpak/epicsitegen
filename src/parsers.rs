use nom::{
  IResult,
  Parser,
  combinator::map,
  multi::many1,
  branch::alt,
  sequence::{preceded},
  bytes::complete::{escaped, is_not, take_until1, tag},
  character::complete::{char, alphanumeric1, multispace0},
};
use crate::template::{TemplateElement};

fn escaped_string(input: &str) -> IResult<&str, &str> { escaped(is_not("\""), '\\', char('"'))(input) }

fn ident_start(input: &str) -> IResult<&str, &str> { alphanumeric1(input) }

fn plain_text_no_open(input: &str) -> IResult<&str, TemplateElement> {
  let (inputparsed, parsed) = take_until1("{")(input)?;
  Ok((inputparsed, TemplateElement::PlainText(parsed.to_string())))
}

fn plain_text_open(input: &str) -> IResult<&str, TemplateElement> {
  let (inputparsed, parsed) = preceded(char('{'), take_until1("{"))(input)?;
  Ok((inputparsed, TemplateElement::PlainTextWithOpen(parsed.to_string())))
}

fn replacement(input: &str) -> IResult<&str, TemplateElement> {
  let (inputparsed, _) = tag("{{")(input)?;
  let (inputparsed, _) = multispace0(inputparsed)?;
  let (inputparsed, name) = ident_start(inputparsed)?;
  let (inputparsed, _) = multispace0(inputparsed)?;
  let (inputparsed, _) = tag("}}")(inputparsed)?;
  Ok((inputparsed, TemplateElement::Replace{identifier: name.to_owned(), pipe: Vec::new()}))
}

pub fn parse_template_elements(input: &str) -> IResult<&str, Vec<TemplateElement>> {
  many1(alt((
        replacement,
        plain_text_no_open,
        plain_text_open
  )))(input)
}
