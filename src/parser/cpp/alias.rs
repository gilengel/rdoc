use crate::parser::cpp::ctype::{CType, parse_cpp_type};
use crate::parser::cpp::template::parse_template;
use crate::parser::{parse_str, ws};
use crate::types::Parsable;
use nom::IResult;
use nom::Parser;
use nom::bytes::complete::tag;
use nom::character::complete::{char, multispace0, multispace1};
use nom::combinator::opt;
use nom::sequence::preceded;
use nom_language::error::VerboseError;

#[derive(Debug, PartialEq)]
pub struct CppAlias<'a> {
    name: &'a str,
    ctype: CType<'a>,
}

impl<'a> Parsable<'a> for CppAlias<'a> {
    fn parse(input: &'a str) -> IResult<&'a str, Self, VerboseError<&'a str>> {
        let (input, _) = opt(parse_template).parse(input)?;
        let (input, _) = tag("using")(input)?;
        let (input, _) = multispace1(input)?;
        let (input, name) = parse_str(input)?;
        let (input, _) = ws(char('=')).parse(input)?;
        let (input, ctype) = parse_cpp_type(input)?;
        let (input, _) = preceded(multispace0, char(';')).parse(input)?;

        Ok((input, CppAlias { name, ctype }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_alias_with_template() {
        let input = r#"template<class T>
                            using Vec = vector<T, Alloc<T>>;"#;
        let result = CppAlias::parse(input);
        assert_eq!(
            result,
            Ok((
                "",
                CppAlias {
                    name: "Vec",
                    ctype: CType::Generic(
                        Box::from(CType::Path(vec!["vector"])),
                        vec![
                            CType::Path(vec!["T"]),
                            CType::Generic(
                                Box::from(CType::Path(vec!["Alloc"])),
                                vec![CType::Path(vec!["T"])]
                            )
                        ]
                    )
                }
            ))
        )
    }

    #[test]
    fn parse_alias_with_namespace() {
        let input = r#"using myNumber = path::subpath::value;"#;
        let result = CppAlias::parse(input);
        assert_eq!(
            result,
            Ok((
                "",
                CppAlias {
                    name: "myNumber",
                    ctype: CType::Path(vec!["path", "subpath", "value"]),
                }
            ))
        )
    }
}
