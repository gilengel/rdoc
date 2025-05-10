use crate::parser::cpp::parse_ws_str;
use nom::branch::alt;
use nom::character::complete::multispace0;
use nom::combinator::{opt, value};
use nom::multi::separated_list1;
use nom::{IResult, Parser, bytes::complete::tag};

#[derive(Debug, Eq, PartialEq, Clone)]
struct CppClass<'a> {
    name: &'a str,
    api: Option<&'a str>,
    parents: Vec<CppParentClass<'a>>,
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
    let (input, _) = tag(":")(input)?;
    let (input, parent_classes) =
        separated_list1(tag(","), parse_single_inheritance).parse(input)?;

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

    let (input, _) = tag("{")(input)?;
    let (input, _) = multispace0(input)?; // skip everything between {} of a class for the moment
    let (input, _) = tag("}")(input)?;

    Ok((
        input,
        CppClass {
            name,
            api,
            parents: parents.unwrap_or(vec![]),
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::parser::cpp::class::{
        CppClass, CppParentClass, InheritanceVisibility, parse_cpp_class,
    };
    use rand::Rng;

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
                        parents: vec![]
                    }
                ))
            );
        }
    }
}
