use crate::parser::cpp::method::{CppFunction, parse_cpp_method};
use crate::parser::cpp::parse_ws_str;
use nom::branch::alt;
use nom::character::complete::{char, multispace0};
use nom::combinator::{opt, value};
use nom::multi::{separated_list0, separated_list1};
use nom::{IResult, Parser, bytes::complete::tag};
use crate::parser::cpp::ctype::CType;

#[derive(Debug, Eq, PartialEq, Clone)]
struct CppClass<'a> {
    name: &'a str,
    api: Option<&'a str>,
    parents: Vec<CppParentClass<'a>>,
    methods: Vec<CppFunction<'a>>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
struct CppParentClass<'a> {
    name: &'a str,
    visibility: InheritanceVisibility,
}

#[derive(Debug, Eq, PartialEq, Clone)]
enum InheritanceVisibility {
    Private,
    Protected,
    Public,
    Empty,
}

impl From<&str> for InheritanceVisibility {
    fn from(value: &str) -> Self {
        match value {
            "private" => InheritanceVisibility::Private,
            "protected" => InheritanceVisibility::Protected,
            "public" => InheritanceVisibility::Public,
            _ => InheritanceVisibility::Empty,
        }
    }
}

fn parse_inheritance_visibility(input: &str) -> IResult<&str, InheritanceVisibility> {
    let (input, _) = multispace0(input)?;

    // Try to match one of the known keywords
    let (input, visibility) = alt((
        value(InheritanceVisibility::Private, tag("private")),
        value(InheritanceVisibility::Protected, tag("protected")),
        value(InheritanceVisibility::Public, tag("public")),
    ))
    // If no match, return Empty without consuming input
    .or(value(
        InheritanceVisibility::Empty,
        nom::combinator::success(()),
    ))
    .parse(input)?;

    Ok((input, visibility))
}

fn parse_single_inheritance(input: &str) -> IResult<&str, CppParentClass> {
    let (input, visibility) = parse_inheritance_visibility(input)?;
    let (input, parent_name) = parse_ws_str(input)?;

    Ok((
        input,
        CppParentClass {
            name: parent_name,
            visibility,
        },
    ))
}
fn parse_inheritance(input: &str) -> IResult<&str, Vec<CppParentClass>> {
    let (input, _) = multispace0(input)?;
    let (input, _) = char(':')(input)?;
    let (input, parent_classes) =
        separated_list1(char(','), parse_single_inheritance).parse(input)?;

    Ok((input, parent_classes))
}

fn parse_cpp_class(input: &str) -> IResult<&str, CppClass> {
    let (input, _) = tag("class")(input)?;
    let (input, maybe_api) = parse_ws_str(input)?;
    let (input, maybe_name_result) = opt(parse_ws_str).parse(input)?;

    let (api, name) = match maybe_name_result {
        Some(name) => (Some(maybe_api), name), // two identifiers → api + name
        None => (None, maybe_api),             // only one → name, no api
    };

    let (input, parents) = opt(parse_inheritance).parse(input)?;

    let (input, _) = char('{')(input)?;
    let (input, _) = multispace0(input)?; // skip everything between {} of a class for the moment

    let (input, methods) = separated_list0(tag("\n"), parse_cpp_method).parse(input)?;
    let (input, _) = char('}')(input)?;

    Ok((
        input,
        CppClass {
            name,
            api,
            parents: parents.unwrap_or(vec![]),
            methods,
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::parser::cpp::class::{
        CppClass, CppParentClass, InheritanceVisibility, parse_cpp_class,
    };
    use crate::parser::cpp::method::CppFunction;
    use rand::Rng;
    use crate::parser::cpp::ctype::CType;

    fn random_whitespace_string() -> String {
        let mut rng = rand::rng();
        let len = rng.random_range(1..=40);
        " ".repeat(len)
    }

    fn random_newline_string() -> String {
        let mut rng = rand::rng();
        let len = rng.random_range(1..=40);
        "\n".repeat(len)
    }

    #[test]
    fn test_parse_empty_cpp_with_inheritance_class() {
        for visibility in ["private", "protected", "public", ""] {
            let input = format!("class test : {visibility} a {{}}");
            assert_eq!(
                parse_cpp_class(&input[..]),
                Ok((
                    "",
                    CppClass {
                        name: "test",
                        api: None,
                        parents: vec![CppParentClass {
                            name: "a",
                            visibility: InheritanceVisibility::from(visibility)
                        }],
                        methods: vec![]
                    }
                ))
            );
        }
    }

    #[test]
    fn test_parse_empty_cpp_with_multiple_inheritance_classes() {
        let input = format!("class test : public a, private b {{}}");
        assert_eq!(
            parse_cpp_class(&input[..]),
            Ok((
                "",
                CppClass {
                    name: "test",
                    api: None,
                    parents: vec![
                        CppParentClass {
                            name: "a",
                            visibility: InheritanceVisibility::Public
                        },
                        CppParentClass {
                            name: "b",
                            visibility: InheritanceVisibility::Private
                        }
                    ],
                    methods: vec![]
                }
            ))
        );
    }

    #[test]
    fn test_parse_empty_cpp_with_api() {
        let input = format!("class MY_API test {{}}");
        assert_eq!(
            parse_cpp_class(&input[..]),
            Ok((
                "",
                CppClass {
                    name: "test",
                    api: Some("MY_API"),
                    parents: vec![],
                    methods: vec![]
                }
            ))
        );
    }

    #[test]
    fn fussy_test_parse_empty_cpp_class() {
        for _ in 0..100 {
            let input = format!(
                "class {}test{}{{ {} }}",
                random_whitespace_string(),
                random_whitespace_string(),
                random_newline_string()
            );
            assert_eq!(
                parse_cpp_class(&input[..]),
                Ok((
                    "",
                    CppClass {
                        name: "test",
                        api: None,
                        parents: vec![],
                        methods: vec![]
                    }
                ))
            );
        }
    }

    #[test]
    fn test_parse_class_with_method() {
        let input = format!("class test {{void hello();}}");
        assert_eq!(
            parse_cpp_class(&input[..]),
            Ok((
                "",
                CppClass {
                    name: "test",
                    api: None,
                    parents: vec![],
                    methods: vec![CppFunction {
                        name: "hello",
                        ..Default::default()
                    }]
                }
            ))
        );
    }

    #[test]
    fn test_parse_class_with_multiple_methods() {
        let input = format!("class test {{void hello();\nvoid goodbye();}}");
        assert_eq!(
            parse_cpp_class(&input[..]),
            Ok((
                "",
                CppClass {
                    name: "test",
                    api: None,
                    parents: vec![],
                    methods: vec![
                        CppFunction {
                            name: "hello",
                            return_type: CType::Path(vec!["void"]),
                            ..Default::default()
                        },
                        CppFunction {
                            name: "goodbye",
                            return_type: CType::Path(vec!["void"]),
                            ..Default::default()
                        }
                    ]
                }
            ))
        );
    }
}

#[test]
fn test_parse_class_with_multiple_mixed_methods() {
    let input = format!("class test {{void hello();\nauto goodbye() -> int;}}");
    assert_eq!(
        parse_cpp_class(&input[..]),
        Ok((
            "",
            CppClass {
                name: "test",
                api: None,
                parents: vec![],
                methods: vec![
                    CppFunction {
                        name: "hello",
                        ..Default::default()
                    },
                    CppFunction {
                        name: "goodbye",
                        return_type: CType::Path(vec!["int"]),
                        ..Default::default()
                    }
                ]
            }
        ))
    );
}
