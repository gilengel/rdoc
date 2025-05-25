use crate::parser::cpp::class::CppClass;
use crate::parser::cpp::comment::CppComment;
use crate::parser::cpp::member::CppMember;
use crate::parser::cpp::method::CppFunction;
use crate::parser::generic::namespace::Namespace;

#[derive(Debug, PartialEq, Clone, Default)]
pub struct CppNamespace<'a> {
    pub name: &'a str,
    pub namespaces: Vec<CppNamespace<'a>>,
    pub classes: Vec<CppClass<'a>>,
    pub functions: Vec<CppFunction<'a>>,
    pub variables: Vec<CppMember<'a>>,
    pub comments: Vec<CppComment>,
}

impl<'a> Namespace<'a, CppClass<'a>> for CppNamespace<'a> {

    fn namespace(
        name: &'a str,
        namespaces: Vec<Self>,
        functions: Vec<CppFunction<'a>>,
        variables: Vec<CppMember<'a>>,
        classes: Vec<CppClass<'a>>,
        comments: Vec<CppComment>,
    ) -> Self
    where
        Self: 'a + Sized,
    {
        CppNamespace {
            name,
            namespaces,
            classes,
            functions,
            variables,
            comments,
        }
    }
}
#[cfg(test)]
mod tests {
    use crate::parser::cpp::class::CppClass;
    use crate::parser::cpp::comment::CppComment;
    use crate::parser::cpp::namespace::CppNamespace;
    use crate::parser::generic::namespace::parse_namespace;

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

        let result = parse_namespace(input);
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

        let result = parse_namespace(input);
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
                comments: vec![CppComment {
                    comment: "some comment".into(),
                }],
                ..Default::default()
            },
        ));

        let result = parse_namespace(input);
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

        let result = parse_namespace(input);
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

        let result = parse_namespace(input);
        assert_eq!(result, expected);
    }
}
