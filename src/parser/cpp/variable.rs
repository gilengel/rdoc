use nom::branch::alt;
use nom::bytes::complete::{escaped_transform, is_not, tag};
use nom::character::complete::multispace0;
use nom::character::complete::{char, i128};
use nom::combinator::{map, map_res, opt, value};
use nom::multi::many0;
use nom::number::complete::float;
use nom::sequence::{delimited, preceded, terminated};
use nom::{IResult, Parser};
use std::str::FromStr;
use nom_language::error::VerboseError;
use crate::parser::cpp::ctype::{parse_cpp_type, CType};
use crate::parser::{parse_str, ws};

#[derive(Debug, Default, PartialEq)]
pub struct CppVariableDecl<'a> {
    name: &'a str,
    ctype: CType<'a>,
    value: Option<Literal>,
    specifiers: Vec<VariableSpecifier>,
}

#[derive(Debug, PartialEq)]
pub enum VariableSpecifier {
    Const,
    Static,
    Constexpr,
    Inline,
}

#[derive(Debug, PartialEq)]
enum Literal {
    Int(i128),
    Flt(f32),
    Str(String),
}

impl FromStr for VariableSpecifier {
    type Err = ();

    fn from_str(specifier: &str) -> Result<Self, Self::Err> {
        match specifier {
            "const" => Ok(Self::Const),
            "static" => Ok(Self::Static),
            "constexpr" => Ok(Self::Constexpr),
            "inline" => Ok(Self::Inline),
            _ => Err(()),
        }
    }
}

fn variable_specifier(input: &str) -> IResult<&str, VariableSpecifier, VerboseError<&str>> {
    // alt(...) returns a parser object
    // map_res(...) wraps it, returning another parser object
    // and *that* object is invoked with `.parse(input)`
    map_res(
        alt((
            terminated(tag("const"), multispace0),
            terminated(tag("static"), multispace0),
            terminated(tag("constexpr"), multispace0),
            terminated(tag("inline"), multispace0),
        )),
        VariableSpecifier::from_str,
    )
    .parse(input) // <-- nom 8 style: call `.parse(...)` on the parser object
}

fn escaped_string(input: &str) -> IResult<&str, String, VerboseError<&str>> {
    delimited(
        char('"'),
        escaped_transform(
            is_not("\\\""),
            '\\',
            alt((
                value("\\", tag("\\")),
                value("\"", tag("\"")),
                value("\n", tag("n")),
                value("\r", tag("r")),
                value("\t", tag("t")),
                // Add more escape sequences if needed
            )),
        ),
        char('"'),
    )
    .parse(input)
}

fn type_value(input: &str) -> IResult<&str, Literal, VerboseError<&str>> {
    // each branch is `map(parser, Literal::*)`
    alt((
        map(i128, Literal::Int),
        map(float, Literal::Flt),
        map(escaped_string, Literal::Str),
    ))
    .parse(input) // now `Choice<…>` *does* implement Parser<_, Literal, _>
}

fn eq_sign(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    map(
        (multispace0, char('='), multispace0),
        |_| "", // ignore the tuple, return empty string
    )
    .parse(input)
}

pub fn variable_decl(input: &str) -> IResult<&str, CppVariableDecl, VerboseError<&str>> {
    let (input, specifiers) = many0(variable_specifier).parse(input)?;
    let (input, ctype) = parse_cpp_type(input)?;
    let (input, _) = multispace0(input)?;
    let (input, name) = parse_str(input)?;
    let (input, _) = multispace0(input)?;
    let (input, value) = opt(preceded(
        opt(eq_sign),
        alt((
            delimited(char('('), ws(type_value), char(')')),
            delimited(char('{'), ws(type_value), char('}')),
            ws(type_value),
        )),
    ))
    .parse(input)?;

    let (input, _) = multispace0(input)?;
    let (input, _) = char(';').parse(input)?;

    Ok((
        input,
        CppVariableDecl {
            name,
            ctype,
            specifiers,
            value,
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::parser::cpp::ctype::CType;
    use crate::parser::cpp::variable::{CppVariableDecl, VariableSpecifier, variable_decl};
    use crate::parser::cpp::variable::Literal::Str;

    #[test]
    fn is_variable_decl() {
        let input = "const auto a = \"hello world\";";

        assert_eq!(
            variable_decl(input),
            Ok((
                "",
                CppVariableDecl {
                    name: "a",
                    ctype: CType::Auto,
                    value: Some(Str("hello world".to_string())),
                    specifiers: vec![VariableSpecifier::Const],
                }
            ))
        );
    }

    #[test]
    fn is_variable_const_static_concrete_decl() {
        let input = "const static FName UE_String = \"hello world\";";

        assert_eq!(
            variable_decl(input),
            Ok((
                "",
                CppVariableDecl {
                    name: "UE_String",
                    ctype: CType::Path(vec!["FName"]),
                    value: Some(Str("hello world".to_string())),
                    specifiers: vec![VariableSpecifier::Const, VariableSpecifier::Static],
                }
            ))
        );
    }
}
