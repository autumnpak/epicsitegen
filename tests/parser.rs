use epicsitegen::parsers::plain_text;
use epicsitegen::template::TemplateElement;
use nom::{Parser, IResult, Err};

fn parser_works(
    parser: fn(&str) -> IResult<&str, TemplateElement>,
    input: &str, 
    leftover: &str, 
    expected: TemplateElement)
{
    assert_eq!(parser(input), Ok((leftover, expected)));
}

#[test]
fn basic() {
    parser_works(plain_text, "yeah{no}{% }", "{% }", TemplateElement::PlainText("yeah{no}".to_string()));
}
