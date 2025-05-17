use nom::branch::alt;
use nom::{
    IResult, Parser,
    bytes::complete::{tag, take_while, take_while1},
    character::complete::{char, digit1, multispace0, multispace1},
    combinator::{map_res, opt, recognize},
    multi::separated_list0,
    sequence::{delimited, pair, preceded, terminated},
};
use nom_language::error::VerboseError;
use crate::parser::cpp::ctype::{parse_cpp_type, CType};

#[derive(Debug, PartialEq)]
pub struct CppEnum<'a> {
    pub name: Option<String>,
    pub variants: Vec<EnumVariant>,
    pub ctype: Option<CType<'a>>,
}

#[derive(Debug, PartialEq)]
pub struct EnumVariant {
    pub name: String,
    pub value: Option<i64>,
}

// Parse C++ identifier: start with alpha or '_', continue alphanumeric or '_'
fn identifier(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    let first_char = |c: char| c.is_ascii_alphabetic() || c == '_';
    let other_char = |c: char| c.is_ascii_alphanumeric() || c == '_';

    recognize(pair(take_while1(first_char), take_while(other_char))).parse(input)
}

// Parse optional integer literal (decimal only for simplicity)
fn int_literal(input: &str) -> IResult<&str, i64, VerboseError<&str>> {
    map_res(recognize(pair(opt(char('-')), digit1)), |s: &str| {
        s.parse::<i64>()
    })
    .parse(input)
}

// Parse one enum variant: identifier [= int_literal]
fn enum_variant(input: &str) -> IResult<&str, EnumVariant, VerboseError<&str>> {
    let (input, name) = identifier(input)?;
    let (input, value) = opt(preceded(
        delimited(multispace0, char('='), multispace0),
        int_literal,
    ))
    .parse(input)?;

    Ok((
        input,
        EnumVariant {
            name: name.to_string(),
            value,
        },
    ))
}

// Parse comma separated list of variants (allow trailing comma)
fn enum_variants(input: &str) -> IResult<&str, Vec<EnumVariant>, VerboseError<&str>> {
    let (input, variants) =
        separated_list0(delimited(multispace0, char(','), multispace0), enum_variant)
            .parse(input)?;

    let (input, _) = opt(char(',')).parse(input)?; // optional trailing comma

    Ok((input, variants))
}

// Parse the full enum
pub fn cpp_enum(input: &str) -> IResult<&str, CppEnum, VerboseError<&str>> {
    let (input, _) = (tag("enum"), multispace1).parse(input)?;
    let (input, _) = opt(delimited(
        multispace0,
        alt((tag("struct"), tag("class"))),
        multispace0,
    ))
    .parse(input)?;
    let (input, name) = opt(terminated(identifier, multispace0)).parse(input)?;
    let (input, ctype) = opt(delimited((char(':'), multispace0), parse_cpp_type, multispace0)).parse(input)?;
    let (input, variants) = delimited(
        char('{'),
        delimited(multispace0, enum_variants, multispace0),
        char('}'),
    )
    .parse(input)?;
    let (input, _) = delimited(multispace0, char(';'), multispace0).parse(input)?;

    Ok((
        input,
        CppEnum {
            name: name.map(|s| s.to_string()),
            variants,
            ctype,
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::parser::cpp::ctype::CType::Path;
    use super::*;

    #[test]
    fn test_identifier() {
        assert_eq!(identifier("foo123 "), Ok((" ", "foo123")));
        assert_eq!(identifier("_bar"), Ok(("", "_bar")));
        assert!(identifier("123abc").is_err());
    }

    #[test]
    fn test_int_literal() {
        assert_eq!(int_literal("123 "), Ok((" ", 123)));
        assert_eq!(int_literal("-45"), Ok(("", -45)));
        assert!(int_literal("abc").is_err());
    }

    #[test]
    fn test_enum_variant() {
        assert_eq!(
            enum_variant("Red"),
            Ok((
                "",
                EnumVariant {
                    name: "Red".to_string(),
                    value: None
                }
            ))
        );
        assert_eq!(
            enum_variant("Green = 5"),
            Ok((
                "",
                EnumVariant {
                    name: "Green".to_string(),
                    value: Some(5)
                }
            ))
        );
    }

    #[test]
    fn test_enum_variants() {
        assert_eq!(
            enum_variants("Red, Green=5 , Blue,"),
            Ok((
                "",
                vec![
                    EnumVariant {
                        name: "Red".into(),
                        value: None
                    },
                    EnumVariant {
                        name: "Green".into(),
                        value: Some(5)
                    },
                    EnumVariant {
                        name: "Blue".into(),
                        value: None
                    },
                ]
            ))
        );
    }

    #[test]
    fn test_cpp_enum() {
        let src = r#"enum class Color : i8 {
                Red,
                Green = 5,
                Blue,
            };
        "#;

        let expected = CppEnum {
            name: Some("Color".to_string()),
            ctype: Some(Path(vec!["i8"])),
            variants: vec![
                EnumVariant {
                    name: "Red".to_string(),
                    value: None,
                },
                EnumVariant {
                    name: "Green".to_string(),
                    value: Some(5),
                },
                EnumVariant {
                    name: "Blue".to_string(),
                    value: None,
                },
            ],
        };

        assert_eq!(cpp_enum(src), Ok(("", expected)));
    }

    #[test]
    fn test_cpp_enum_no_name() {
        let src = r#"enum
        {
            Foo = 10,
            Bar,
        };
        "#;

        let expected = CppEnum {
            name: None,
            ctype: None,
            variants: vec![
                EnumVariant {
                    name: "Foo".to_string(),
                    value: Some(10),
                },
                EnumVariant {
                    name: "Bar".to_string(),
                    value: None,
                },
            ],
        };

        assert_eq!(cpp_enum(src), Ok(("", expected)));
    }
}
