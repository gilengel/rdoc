use crate::parser::cpp::comment::CppComment;
use crate::parser::cpp::member::CppMember;
use crate::parser::cpp::method::CppFunction;

use crate::parser::generic::class::{Class, CppParentClass, InheritanceVisibility, parse_class};

use crate::parser::generic::annotation::NoAnnotation;
use std::collections::HashMap;
use crate::parser::cpp::ctype::CType::Path;

#[derive(Debug, PartialEq, Clone)]
pub struct CppClass<'a> {
    pub name: &'a str,
    pub api: Option<&'a str>,
    pub parents: Vec<CppParentClass<'a>>,
    pub methods: HashMap<InheritanceVisibility, Vec<CppFunction<'a>>>,
    pub members: HashMap<InheritanceVisibility, Vec<CppMember<'a>>>,
    pub inner_classes: HashMap<InheritanceVisibility, Vec<CppClass<'a>>>,
}

impl Default for CppClass<'_> {
    fn default() -> Self {
        Self {
            name: "",
            api: None,
            parents: vec![],
            methods: HashMap::from([]),
            members: HashMap::from([]),
            inner_classes: HashMap::from([]),
        }
    }
}

impl<'a>
    Class<'a, NoAnnotation, CppFunction<'a>, NoAnnotation, CppMember<'a>, NoAnnotation, CppComment>
    for CppClass<'a>
{
    fn class(
        name: &'a str,
        api: Option<&'a str>,
        parents: Vec<CppParentClass<'a>>,
        methods: HashMap<InheritanceVisibility, Vec<CppFunction<'a>>>,
        members: HashMap<InheritanceVisibility, Vec<CppMember<'a>>>,
        inner_classes: HashMap<InheritanceVisibility, Vec<CppClass<'a>>>,
        _annotation: Option<Vec<NoAnnotation>>,
    ) -> Self
    where
        Self: 'a,
    {
        Self {
            name,
            api,
            parents,
            methods,
            members,
            inner_classes,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::cpp::class::{CppClass, CppParentClass, InheritanceVisibility};

    use crate::parser::cpp::ctype::CType::Path;
    use crate::parser::cpp::method::CppFunction;
    use crate::parser::generic::class::parse_class;
    use rand::Rng;
    use std::collections::HashMap;

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
            let input = format!("class test : {visibility} a {{}};");
            let result = parse_class(&input, &vec![]);

            assert_eq!(
                result,
                Ok((
                    "",
                    CppClass {
                        name: "test",
                        api: None,
                        parents: vec![CppParentClass {
                            name: Path(vec!["a"]),
                            visibility: InheritanceVisibility::from(visibility)
                        }],
                        ..CppClass::default()
                    }
                ))
            );
        }
    }

    #[test]
    fn test_empty_struct() {
        let input = "struct Test {};";
        let result = parse_class(&input, &vec![]);
        assert_eq!(
            result,
            Ok((
                "",
                CppClass {
                    name: "Test",
                    ..CppClass::default()
                }
            ))
        );
    }
    #[test]
    fn test_ignore_template_specialisation() {
        let input = r#"template<>
        struct Test<int>
        {
        };"#;
        let result = parse_class(&input, &vec![]);

        assert_eq!(
            result,
            Ok((
                "",
                CppClass {
                    name: "Test",
                    ..CppClass::default()
                }
            ))
        );
    }

    #[test]
    fn test_parse_empty_struct() {
        let input = "struct Test {};";
        let result = parse_class(&input, &vec![]);

        assert_eq!(
            result,
            Ok((
                "",
                CppClass {
                    name: "Test",
                    ..CppClass::default()
                }
            ))
        );
    }

    #[test]
    fn test_parse_nested_struct() {
        let input = r#"struct Test {
            struct Inner {}
        };"#;
        let result = parse_class(&input, &vec![]);

        assert_eq!(
            result,
            Ok((
                "",
                CppClass {
                    name: "Test",
                    inner_classes: HashMap::from([(
                        InheritanceVisibility::Private,
                        vec![CppClass {
                            name: "Inner",
                            ..CppClass::default()
                        }]
                    ),]),
                    ..CppClass::default()
                }
            ))
        );
    }

    #[test]
    fn test_parse_struct_with_constructor() {
        let input = r#"struct Test
        {
            Test(){};
        };"#;
        let result = parse_class(&input, &vec![]);

        assert_eq!(
            result,
            Ok((
                "",
                CppClass {
                    name: "Test",
                    methods: HashMap::from([(
                        InheritanceVisibility::Private,
                        vec![CppFunction {
                            name: "Test",
                            ..Default::default()
                        }]
                    ),]),
                    ..CppClass::default()
                }
            ))
        );
    }

    #[test]
    fn test_parse_empty_templated_struct() {
        let input = "template<typename T>\nstruct Test {};";
        let result = parse_class(&input, &vec![]);

        assert_eq!(
            result,
            Ok((
                "",
                CppClass {
                    name: "Test",
                    ..CppClass::default()
                }
            ))
        );
    }

    #[test]
    fn test_parse_empty_cpp_with_multiple_inheritance_classes() {
        let input = "class test : public a, private b {};";
        let result = parse_class(&input, &vec![]);

        assert_eq!(
            result,
            Ok((
                "",
                CppClass {
                    name: "test",
                    api: None,
                    parents: vec![
                        CppParentClass {
                            name: Path(vec!["a"]),
                            visibility: InheritanceVisibility::Public
                        },
                        CppParentClass {
                            name: Path(vec!["b"]),
                            visibility: InheritanceVisibility::Private
                        }
                    ],
                    ..CppClass::default()
                }
            ))
        );
    }

    #[test]
    fn test_parse_empty_cpp_with_namespaced_inheritance_class() {
        let input = "class test : public namespace::a {};";
        let result = parse_class(&input, &vec![]);

        assert_eq!(
            result,
            Ok((
                "",
                CppClass {
                    name: "test",
                    api: None,
                    parents: vec![CppParentClass {
                        name: Path(vec!["namespace", "a"]),
                        visibility: InheritanceVisibility::Public
                    }],
                    ..CppClass::default()
                }
            ))
        );
    }

    #[test]
    fn test_parse_class_with_constructor() {
        let input1 = r#"class test {
            public:
                test()
                {
                    int j = 42;
                    for(int i = 0; i < j; i++)
                    {
                        std::cout << j << std::endl;
                    }
                }
        };"#;

        let input2 = r#"class test {
            public:
                test();
        };"#;

        for input in [input1, input2] {
            let result = parse_class(&input, &vec![]);

            assert_eq!(
                result,
                Ok((
                    "",
                    CppClass {
                        name: "test",
                        methods: HashMap::from([(
                            InheritanceVisibility::Public,
                            vec![CppFunction {
                                name: "test",
                                ..Default::default()
                            }]
                        ),]),
                        ..CppClass::default()
                    }
                ))
            );
        }
    }

    #[test]
    fn test_parse_empty_cpp_with_api() {
        let input = format!("class MY_API test {{}};");
        let result = parse_class(&input, &vec![]);

        assert_eq!(
            result,
            Ok((
                "",
                CppClass {
                    name: "test",
                    api: Some("MY_API"),
                    ..CppClass::default()
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
            let result = parse_class(&input, &vec![]);

            assert_eq!(
                result,
                Ok((
                    "",
                    CppClass {
                        name: "test",
                        ..CppClass::default()
                    }
                ))
            );
        }
    }

    #[test]
    fn test_parse_class_with_method() {
        let input = r#"class test {
                void hello();
            };"#;
        let result = parse_class(&input, &vec![]);

        assert_eq!(
            result,
            Ok((
                "",
                CppClass {
                    name: "test",
                    methods: HashMap::from([(
                        InheritanceVisibility::Private,
                        vec![CppFunction {
                            name: "hello",
                            ..Default::default()
                        }]
                    ),]),
                    ..CppClass::default()
                }
            ))
        );
    }

    #[test]
    fn test_parse_class_with_multiple_methods() {
        let input = "class test {void hello();\nvoid goodbye();};";
        let result = parse_class(&input, &vec![]);

        assert_eq!(
            result,
            Ok((
                "",
                CppClass {
                    name: "test",
                    methods: HashMap::from([(
                        InheritanceVisibility::Private,
                        vec![
                            CppFunction {
                                name: "hello",
                                return_type: None,
                                ..Default::default()
                            },
                            CppFunction {
                                name: "goodbye",
                                return_type: None,
                                ..Default::default()
                            }
                        ]
                    ),]),
                    ..CppClass::default()
                }
            ))
        );
    }
}

#[test]
fn test_parse_class_with_multiple_mixed_methods() {
    let input = format!("class test {{void hello();\nauto goodbye() -> int;}}");
    let result = parse_class(&input, &vec![]);

    assert_eq!(
        result,
        Ok((
            "",
            CppClass {
                name: "test",
                methods: HashMap::from([(
                    InheritanceVisibility::Private,
                    vec![
                        CppFunction {
                            name: "hello",
                            ..Default::default()
                        },
                        CppFunction {
                            name: "goodbye",
                            return_type: Some(Path(vec!["int"])),
                            ..Default::default()
                        }
                    ]
                ),]),
                ..CppClass::default()
            }
        ))
    );
}

#[test]
fn test_simple_class() {
    let input = r#"class TestClass {
        private:
            // says only hello to itself
            auto helloPrivate() -> void;

        protected:
            // says only hello to its relatives
            auto helloProtected() -> void;

        public:
            /*
             * says hello to everybody that listens
             */
            auto hello() -> void;

        private:
            /// internal counter on how many times others were greeted
            int count{0};
    };"#;

    let result = parse_class(input, &vec![]);

    assert_eq!(
        result,
        Ok((
            "",
            CppClass {
                name: "TestClass",
                methods: HashMap::from([
                    (
                        InheritanceVisibility::Private,
                        vec![CppFunction {
                            name: "helloPrivate",
                            comment: Some(CppComment {
                                comment: "says only hello to itself".to_string(),
                            }),
                            ..Default::default()
                        }],
                    ),
                    (
                        InheritanceVisibility::Protected,
                        vec![CppFunction {
                            name: "helloProtected",
                            comment: Some(CppComment {
                                comment: "says only hello to its relatives".to_string(),
                            }),
                            ..Default::default()
                        }],
                    ),
                    (
                        InheritanceVisibility::Public,
                        vec![CppFunction {
                            name: "hello",
                            comment: Some(CppComment {
                                comment: "says hello to everybody that listens".to_string(),
                            }),
                            ..Default::default()
                        }],
                    ),
                ]),
                members: HashMap::from([(
                    InheritanceVisibility::Private,
                    vec![CppMember {
                        name: "count",
                        ctype: Path(vec!["int"]),
                        default_value: Some(Path(vec!["0"])),
                        comment: Some(CppComment {
                            comment: "internal counter on how many times others were greeted"
                                .to_string(),
                        }),
                        ..Default::default()
                    }],
                ),]),
                ..CppClass::default()
            }
        ))
    );
}
