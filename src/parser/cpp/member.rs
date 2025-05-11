use crate::parser::cpp::comment::{CppComment, parse_cpp_comment};
use crate::parser::cpp::ctype::{CType, parse_cpp_type};
use crate::parser::cpp::parse_ws_str;
use nom::branch::alt;
use nom::character::complete::{char, multispace0};
use nom::combinator::opt;
use nom::sequence::{delimited, preceded};
use nom::{IResult, Parser};

#[derive(Debug, Eq, PartialEq, Clone, Default)]
pub struct CppMember<'a> {
    pub name: &'a str,
    pub ctype: CType<'a>,
    pub default_value: Option<CType<'a>>,
    pub is_const: bool,
    pub comment: Option<CppComment>,
}

pub fn parse_cpp_member(input: &str) -> IResult<&str, CppMember> {
    let (input, comment) = opt(parse_cpp_comment).parse(input)?;
    let (input, _) = multispace0.parse(input)?;
    let (input, ctype) = parse_cpp_type(input)?;
    let (input, name) = parse_ws_str(input)?;
    let (input, _) = multispace0.parse(input)?;

    let (input, default_value) = opt(alt((
        delimited(char('{'), delimited(multispace0, parse_cpp_type, multispace0), char('}')),
        preceded(char('='), delimited(multispace0, parse_cpp_type, multispace0)),
    )))
    .parse(input)?;

    Ok((
        input,
        CppMember {
            name,
            ctype,
            default_value,
            is_const: false,
            comment,
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::parser::cpp::ctype::CType::Path;
    use crate::parser::cpp::member::{CppMember, parse_cpp_member};

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
                    default_value: None,
                    is_const: false,
                    comment: None,
                }
            ))
        );
    }

    #[test]
    fn test_cpp_member_with_default_value() {
        for input in ["int member = 0", "int member {0}"]
        {
            assert_eq!(
                parse_cpp_member(&input[..]),
                Ok((
                    "",
                    CppMember {
                        name: "member",
                        ctype: Path(vec!["int"]),
                        default_value: Some(Path(vec!["0"])),
                        is_const: false,
                        comment: None,
                    }
                ))
            );
        }

    }
}
