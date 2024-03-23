use nom::{
  IResult,
  multi::many1,
  branch::{alt},
  sequence::{preceded},
  bytes::complete::{escaped, is_not},
  character::complete::{char, alphanumeric1},
};

pub fn escaped_string(input: &str) -> IResult<&str, &str> { escaped(is_not("\""), '\\', char('"'))(input) }

pub fn ident_start(input: &str) -> IResult<&str, &str> { alphanumeric1(input) }

pub fn plain_text_no_open(input: &str) -> IResult<&str, Vec<&str>> { many1(is_not("{"))(input) }
pub fn plain_text_open(input: &str) -> IResult<&str, Vec<&str>> {
  preceded(char('{'), many1(is_not("{%")))(input)
}

//pub fn plain_text(input: &str) -> IResult<&str, Vec<&str>> { many1(alt((plain_text_no_open, plain_text_open)))(input) }
