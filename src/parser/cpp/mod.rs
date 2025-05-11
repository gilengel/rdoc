use nom::bytes::complete::{escaped, take_while1};
use nom::character::complete::{multispace0, one_of};
use nom::{IResult, Parser, error::ParseError, sequence::delimited};

mod class;
mod method;
mod ctype;
mod comment;
mod member;

pub fn ws<'a, O, E: ParseError<&'a str>, F>(inner: F) -> impl Parser<&'a str, Output = O, Error = E>
where
    F: Parser<&'a str, Output = O, Error = E>,
{
    delimited(multispace0, inner, multispace0)
}


fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

fn extended_identifier<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    take_while1(is_ident_char).parse(i)
}

fn parse_str<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    escaped(extended_identifier::<E>, '\\', one_of("\"n\\")).parse(i)
}

fn parse_ws_str<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    ws(parse_str).parse(i)
}