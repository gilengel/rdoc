use nom::character::complete::{multispace0, one_of};
use nom::error::ParseError;
use nom::{IResult, Parser};
use nom::bytes::complete::{escaped, take_while1};
use nom::sequence::delimited;

pub mod cpp;

pub mod ue;
pub mod generic;

pub fn ws<'a, O, E: ParseError<&'a str>, F>(inner: F) -> impl Parser<&'a str, Output=O, Error=E>
where
    F: Parser<&'a str, Output=O, Error=E>,
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

/*

pub fn verbose_alt<'a, O, E: nom::error::ParseError<&'a str>, F>(mut parsers: Vec<F>) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: FnMut(&'a str) -> IResult<&'a O, E>,
{
    move |input| {
        let mut errors = Vec::new();
        for parser in &mut parsers {
            match parser(input) {
                Ok(result) => return Ok(result),
                Err(nom::Err::Error(e)) => errors.push(e),
                Err(e) => return Err(e),
            }
        }

        if let Some(e) = errors.last() {
            for (i, error) in errors.iter().enumerate() {
                eprintln!("alt branch {}: {:?}", i, error);
            }
            Err(nom::Err::Error(e.clone()))
        } else {
            Err(nom::Err::Error(E::from_error_kind(input, nom::error::ErrorKind::Alt)))
        }
    }
}
*/