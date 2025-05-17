use nom::bytes::complete::tag;
use nom::character::complete::{char, multispace0};
use nom::combinator::opt;
use nom::{IResult, Parser};
use nom::branch::alt;
use nom::multi::separated_list0;
use nom_language::error::VerboseError;
use crate::parser::cpp::ctype::{parse_cpp_type, CType};
use crate::parser::ws;

fn parse_template_param(input: &str) -> IResult<&str, CType, VerboseError<&str>> {
    let (input, _) = ws(alt((tag("typename"), tag("class")))).parse(input)?;
    let (input, ctype) = parse_cpp_type(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = opt((ws(char('=')), multispace0, parse_cpp_type)).parse(input)?;

    Ok((input, ctype))
}
pub fn parse_template(input: &str) -> IResult<&str, Vec<CType>, VerboseError<&str>> {
    let (input, _) = ws(tag("template")).parse(input)?;
    let (input, _) = char('<').parse(input)?;
    let (input, params) = separated_list0(tag(","), parse_template_param).parse(input)?;
    let (input, _) = ws(char('>')).parse(input)?;

    Ok((input, params))
}