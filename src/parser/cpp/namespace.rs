use crate::parser::cpp::class::CppClass;
use crate::parser::cpp::method::CppFunction;
use nom::IResult;
use nom::bytes::complete::tag;
use nom::character::complete::{char, multispace0};
use crate::parser::cpp::comment::CppComment;
use crate::parser::parse_ws_str;
use crate::types::Parsable;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct CppNamespace<'a> {
    pub name: &'a str,
    pub namespaces: Vec<CppNamespace<'a>>,
    pub classes: Vec<CppClass<'a>>,
    pub functions: Vec<CppFunction<'a>>,
}

fn parse_inner_namespace(input: &str) -> IResult<&str, CppNamespace> {
    let (input, _) = tag("namespace")(input)?;
    let (input, name) = parse_ws_str(input)?;
    let (input, _) = char('{')(input)?;

    let mut input = input;

    let mut namespace = CppNamespace {
        name,
        ..Default::default()
    };

    loop {
        let (i, _) = multispace0(input)?;
        input = i;

        if let Ok((i, inner_namespace)) = parse_inner_namespace(input) {
            namespace.namespaces.push(inner_namespace);

            input = i;
            continue;
        }


        if let Ok((i, _)) = <CppComment as Parsable>::parse(input) {
            input = i;
            continue;
        }

        if let Ok((i, class)) = <CppClass as Parsable>::parse(input) {
            namespace.classes.push(class);

            input = i;
            continue;
        }

        if let Ok((i, _)) = char::<_, nom::error::Error<&str>>('}')(input) {
            return Ok((i, namespace));
        }

        return Err(nom::Err::Error(nom::error::make_error(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }
}

pub fn parse_cpp_namespace(input: &str) -> IResult<&str, CppNamespace> {
    parse_inner_namespace(input)
}

#[cfg(test)]
mod tests {
    use crate::parser::cpp::class::CppClass;
    use crate::parser::cpp::namespace::{CppNamespace, parse_cpp_namespace};

    #[test]
    fn empty_namespace() {
        let input = "namespace test {}";
        let expected = Ok((
            "",
            CppNamespace {
                name: "test",
                ..Default::default()
            },
        ));

        let result = parse_cpp_namespace(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn empty_nested_namespace() {
        let input = "namespace OuterNamespace { namespace InnerNamespace {} }";
        let expected = Ok((
            "",
            CppNamespace {
                name: "OuterNamespace",
                namespaces: vec![CppNamespace {
                    name: "InnerNamespace",
                    ..Default::default()
                }],
                ..Default::default()
            },
        ));

        let result = parse_cpp_namespace(input);
        assert_eq!(result, expected);
    }
    #[test]
    fn empty_namespace_with_comment() {
        let input = r#"namespace test {
            // some comment
        }"#;

        let expected = Ok((
            "",
            CppNamespace {
                name: "test",
                ..Default::default()
            },
        ));

        let result = parse_cpp_namespace(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn empty_namespace_with_empty_class() {
        let input = r#"namespace test {
            class TestClass {};
        }"#;

        let expected = Ok((
            "",
            CppNamespace {
                name: "test",
                classes: vec![CppClass {
                    name: "TestClass",
                    ..Default::default()
                }],
                ..Default::default()
            },
        ));

        let result = parse_cpp_namespace(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn empty_namespace_with_forward_declaration() {
        let input = r#"namespace test {
            class TestClass;
        }"#;

        let expected = Ok((
            "",
            CppNamespace {
                name: "test",
                classes: vec![CppClass {
                    name: "TestClass",
                    ..Default::default()
                }],
                ..Default::default()
            },
        ));

        let result = parse_cpp_namespace(input);
        assert_eq!(result, expected);
    }
}
