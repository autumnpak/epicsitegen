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

pub fn plain_text_no_open(input: &str) -> IResult<&str, TemplateElement> {
  let (inputparsed, parsed) = take_until1("{")(input)?;
  Ok((inputparsed, TemplateElement::PlainText(parsed.to_string())))
}

pub fn plain_text_open(input: &str) -> IResult<&str, TemplateElement> {
  let (inputparsed, parsed) = preceded(char('{'), take_until1("{%"))(input)?;
  Ok((inputparsed, TemplateElement::PlainTextWithOpen(parsed.to_string())))
}

pub fn template_elements(input: &str) -> IResult<&str, Vec<TemplateElement>> {
  many1(alt((
        plain_text_no_open,
        plain_text_open
  )))(input)
}
