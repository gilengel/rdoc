use crate::parser::cpp::comment::CppComment;
use crate::parser::cpp::ctype::{CType, parse_cpp_type};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{char, multispace0};
use nom::combinator::{map, opt};
use nom::multi::many0;
use nom::sequence::{delimited, preceded};
use nom::{IResult, Parser};
use nom_language::error::VerboseError;
use crate::parser::parse_ws_str;
use crate::types::Parsable;

#[derive(Debug, Eq, PartialEq, Clone, Default)]
pub struct CppMember<'a> {
    pub name: &'a str,
    pub ctype: CType<'a>,
    pub default_value: Option<CType<'a>>,
    pub comment: Option<CppComment>,
    pub modifiers: Vec<CppMemberModifier>,
}

#[derive(Debug, Eq, PartialEq, Clone)]

pub enum CppMemberModifier {
    Static,
    Const,
    Inline,
}

impl Into<String> for CppMemberModifier {
    fn into(self) -> String {
        match self {
            CppMemberModifier::Static => "static".to_string(),
            CppMemberModifier::Const => "const".to_string(),
            CppMemberModifier::Inline => "inline".to_string(),
        }
    }
}
impl From<&str> for CppMemberModifier {
    fn from(value: &str) -> Self {
        match value {
            "static" => CppMemberModifier::Static,
            "const" => CppMemberModifier::Const,
            "inline" => CppMemberModifier::Inline,
            _ => unimplemented!(),
        }
    }
}

fn parse_modifier(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    preceded(
        multispace0,
        alt((tag("static"), tag("const"), tag("inline"))),
    )
    .parse(input)
}

fn parse_modifiers(input: &str) -> IResult<&str, Vec<CppMemberModifier>, VerboseError<&str>> {
    many0(map(parse_modifier, |x| CppMemberModifier::from(x))).parse(input)
}

pub fn parse_cpp_member(input: &str) -> IResult<&str, CppMember, VerboseError<&str>> {
    let (input, comment) = opt(|i| <CppComment as Parsable>::parse(i)).parse(input)?;
    let (input, _) = multispace0.parse(input)?;
    let (input, modifiers) = parse_modifiers(input)?;
    let (input, _) = multispace0.parse(input)?;
    let (input, ctype) = parse_cpp_type(input)?;
    let (input, name) = parse_ws_str(input)?;
    let (input, _) = multispace0.parse(input)?;

    let (input, default_value) = opt(alt((
        delimited(
            char('{'),
            delimited(multispace0, parse_cpp_type, multispace0),
            char('}'),
        ),
        preceded(
            char('='),
            delimited(multispace0, parse_cpp_type, multispace0),
        ),
    )))
    .parse(input)?;

    Ok((
        input,
        CppMember {
            name,
            ctype,
            default_value,
            comment,
            modifiers,
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::parser::cpp::ctype::CType::Path;
    use crate::parser::cpp::member::{CppMember, CppMemberModifier, parse_cpp_member};

    #[test]
    fn test_cpp_member_without_default_value() {
        let input = "int member";
        assert_eq!(
            parse_cpp_member(&input[..]),
            Ok((
                "",
                CppMember {
                    name: "member",
                    ctype: Path(vec!["int"]),
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_cpp_member_with_modifier() {
        for modifier in vec!["static", "const", "inline"] {
            let input = format!("{} int member", modifier);
            assert_eq!(
                parse_cpp_member(&input[..]),
                Ok((
                    "",
                    CppMember {
                        name: "member",
                        ctype: Path(vec!["int"]),
                        modifiers: vec![CppMemberModifier::from(modifier)],
                        ..Default::default()
                    }
                ))
            );
        }
    }

    #[test]
    fn test_cpp_member_with_default_value() {
        for input in ["int member = 0", "int member {0}"] {
            assert_eq!(
                parse_cpp_member(&input[..]),
                Ok((
                    "",
                    CppMember {
                        name: "member",
                        ctype: Path(vec!["int"]),
                        default_value: Some(Path(vec!["0"])),
                        ..Default::default()
                    }
                ))
            );
        }
    }
}
