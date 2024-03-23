mod template;
mod parsers;
use crate::parsers::plain_text_no_open;

fn main() {
    println!("Hello, world!");
    assert_eq!(plain_text_no_open("yeah{no}{% }"), Ok(("", vec!["yeah", "no}"])));
}
