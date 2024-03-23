use nom::{
  IResult,
  Parser,
  combinator::map,
  multi::many1,
  branch::alt,
  sequence::{preceded},
  bytes::complete::{escaped, is_not, take_until1},
  character::complete::{char, alphanumeric1},
};

use crate::template::{TemplateElement};

pub fn escaped_string(input: &str) -> IResult<&str, &str> { escaped(is_not("\""), '\\', char('"'))(input) }

pub fn ident_start(input: &str) -> IResult<&str, &str> { alphanumeric1(input) }

pub fn plain_text_no_open(input: &str) -> IResult<&str, String> {
  let parsed = take_until1("{")(input)?;
  Ok((parsed.0, parsed.1.to_string()))
}

pub fn plain_text_open(input: &str) -> IResult<&str, String> {
  let parsed = preceded(char('{'), take_until1("{%"))(input)?;
  Ok((parsed.0, "{".to_string() + parsed.1))
}

pub fn plain_text(input: &str) -> IResult<&str, TemplateElement> { 
  let parsed = many1(alt((plain_text_no_open, plain_text_open)))(input)?;
  Ok((parsed.0, TemplateElement::PlainText(parsed.1.concat())))
}
