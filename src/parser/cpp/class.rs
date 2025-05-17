use crate::parser::cpp::comment::CppComment;
use crate::parser::cpp::ctype::CType::Path;
use crate::parser::cpp::ctype::{CType, parse_cpp_type};
use crate::parser::cpp::member::{CppMember, parse_cpp_member};
use crate::parser::cpp::method::CppFunction;
use crate::parser::cpp::template::parse_template;
use crate::parser::{parse_ws_str, ws};
use crate::types::Parsable;
use nom::branch::alt;
use nom::bytes::complete::take_until;
use nom::character::complete::{char, multispace0};
use nom::combinator::{map, opt, value};
use nom::multi::separated_list1;
use nom::sequence::{delimited, preceded, terminated};
use nom::{IResult, Parser, bytes::complete::tag};
use std::collections::HashMap;
use nom_language::error::VerboseError;

#[derive(Debug, PartialEq, Clone)]
pub struct CppClass<'a> {
    pub name: &'a str,
    pub api: Option<&'a str>,
    pub parents: Vec<CppParentClass<'a>>,
    pub methods: HashMap<InheritanceVisibility, Vec<CppFunction<'a>>>,
    pub members: HashMap<InheritanceVisibility, Vec<CppMember<'a>>>,
}

impl Default for CppClass<'_> {
    fn default() -> Self {
        Self {
            name: "",
            api: None,
            parents: vec![],
            methods: HashMap::from([
                (InheritanceVisibility::Public, vec![]),
                (InheritanceVisibility::Protected, vec![]),
                (InheritanceVisibility::Private, vec![]),
            ]),
            members: HashMap::from([
                (InheritanceVisibility::Public, vec![]),
                (InheritanceVisibility::Protected, vec![]),
                (InheritanceVisibility::Private, vec![]),
            ]),
        }
    }
}

impl<'a> Parsable<'a> for CppClass<'a>
{
    fn parse(input: &'a str) -> IResult<&'a str, Self, VerboseError<&'a str>> {
        let (input, _) = opt(parse_template).parse(input)?;
        let (input, _) = alt((tag("class"), tag("struct"))).parse(input)?;
        let (input, maybe_api) = parse_ws_str(input)?;
        let (input, maybe_name_result) = opt(parse_ws_str).parse(input)?;

        let (api, name) = match maybe_name_result {
            Some(name) => (Some(maybe_api), name), // two identifiers → api + name
            None => (None, maybe_api),             // only one → name, no api
        };

        // ignore template specialisation atm
        let mut input = input;
        if let Ok((i, _)) = opt(delimited(
            char::<_, nom::error::Error<&str>>('<'),
            take_until(">"),
            char('>'),
        ))
        .parse(input)
        {
            input = i;
        }
        let (input, _) = multispace0(input)?;

        // Return early for empty classes (e.g. forward declaration)
        if let Ok((input, empty)) = opt(char::<_, nom::error::Error<&str>>(';')).parse(input) {
            if empty.is_some() {
                return Ok((
                    input,
                    CppClass {
                        name,
                        ..Default::default()
                    },
                ));
            }
        }

        let (input, parents) = opt(parse_inheritance).parse(input)?;

        let mut methods: HashMap<InheritanceVisibility, Vec<CppFunction>> = HashMap::from([
            (InheritanceVisibility::Private, vec![]),
            (InheritanceVisibility::Protected, vec![]),
            (InheritanceVisibility::Public, vec![]),
        ]);

        let mut members: HashMap<InheritanceVisibility, Vec<CppMember>> = HashMap::from([
            (InheritanceVisibility::Private, vec![]),
            (InheritanceVisibility::Protected, vec![]),
            (InheritanceVisibility::Public, vec![]),
        ]);

        let (input, _) = char('{')(input)?;
        let (mut input, _) = multispace0(input)?; // skip everything between {} of a class for the moment

        let mut current_access = InheritanceVisibility::Private;
        loop {
            let (i, _) = multispace0(input)?;
            input = i;

            if let Ok((next, _)) = parse_ignore(input) {
                input = next;
                continue;
            }

            if let Ok((next, _)) = char::<_, nom::error::Error<&str>>(';')(input) {
                input = next;
                continue;
            }

            if let Ok((next, _)) = char::<_, nom::error::Error<&str>>('}')(input) {
                let (rest, _) = opt(char(';')).parse(next)?;
                return Ok((
                    rest,
                    CppClass {
                        name,
                        api,
                        parents: parents.unwrap_or(vec![]),
                        methods,
                        members,
                    },
                ));
            }

            if let Ok((next_input, access)) = access_specifier(input) {
                current_access = access;
                input = next_input;
                continue;
            }

            if let Ok((next_input, method)) = CppFunction::parse(input) {
                let (next_input, _) =
                    opt(preceded(ws(char(':')), take_until("{").or(take_until(";"))))
                        .parse(next_input)?;

                methods.get_mut(&current_access).unwrap().push(method);
                input = next_input;
                continue;
            }

            if let Ok((next_input, member)) = parse_cpp_member(input) {
                members.get_mut(&current_access).unwrap().push(member);
                input = next_input;
                continue;
            }

            // Skip unknown or unsupported lines (e.g., whitespace or preprocessor)
            if let Ok((next_input, _)) = skip_to_next_line(input) {
                input = next_input;
            } else {
                break;
            }
        }

        Err(nom::Err::Error(nom_language::error::VerboseError { errors: vec![] }))
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct CppParentClass<'a> {
    name: CType<'a>,
    visibility: InheritanceVisibility,
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum InheritanceVisibility {
    Private,
    Protected,
    Public,
    Virtual,
    Empty,
}

impl From<&str> for InheritanceVisibility {
    fn from(value: &str) -> Self {
        match value {
            "private" => InheritanceVisibility::Private,
            "protected" => InheritanceVisibility::Protected,
            "public" => InheritanceVisibility::Public,
            "virtual" => InheritanceVisibility::Virtual,
            _ => InheritanceVisibility::Empty,
        }
    }
}

fn parse_inheritance_visibility(input: &str) -> IResult<&str, InheritanceVisibility, VerboseError<&str>> {
    let (input, _) = multispace0(input)?;

    // Try to match one of the known keywords
    let (input, visibility) = alt((
        value(InheritanceVisibility::Private, tag("private")),
        value(InheritanceVisibility::Protected, tag("protected")),
        value(InheritanceVisibility::Public, tag("public")),
        value(InheritanceVisibility::Virtual, tag("virtual")),
    ))
    // If no match, return Empty without consuming input
    .or(value(
        InheritanceVisibility::Empty,
        nom::combinator::success(()),
    ))
    .parse(input)?;

    Ok((input, visibility))
}

fn parse_single_inheritance(input: &str) -> IResult<&str, CppParentClass, VerboseError<&str>> {
    let (input, visibility) = parse_inheritance_visibility(input)?;
    let (input, name) = ws(parse_cpp_type).parse(input)?;

    Ok((input, CppParentClass { name, visibility }))
}
fn parse_inheritance(input: &str) -> IResult<&str, Vec<CppParentClass>, VerboseError<&str>> {
    let (input, _) = multispace0(input)?;
    let (input, _) = char(':')(input)?;
    let (input, parent_classes) =
        separated_list1(char(','), parse_single_inheritance).parse(input)?;

    Ok((input, parent_classes))
}

fn access_specifier(input: &str) -> IResult<&str, InheritanceVisibility> {
    let (input, specifier) = map(
        terminated(
            alt((tag("public"), tag("private"), tag("protected"))),
            (tag(":"), multispace0),
        ),
        |t| InheritanceVisibility::from(t),
    )
    .parse(input)?;

    Ok((input, specifier))
}

fn skip_to_next_line(input: &str) -> IResult<&str, (), nom::error::Error<&str>> {
    let (input, _) = take_until::<_, _, nom::error::Error<&str>>("\n")(input)?;
    let input = input.strip_prefix('\n').unwrap_or(input);
    Ok((input, ()))
}

pub fn parse_uproperty(input: &str) -> IResult<&str, &str> {
    let (input, _) = (ws(tag("UPROPERTY")), nom::bytes::take_until("\n")).parse(input)?;

    Ok((input, ""))
}

pub fn parse_ufunction(input: &str) -> IResult<&str, &str> {
    let (input, _) = (ws(tag("UFUNCTION")), nom::bytes::take_until("\n")).parse(input)?;

    Ok((input, ""))
}

pub fn parse_generate_body(input: &str) -> IResult<&str, &str> {
    let (input, _) = ws(tag("GENERATE_BODY()")).parse(input)?;

    Ok((input, ""))
}
pub fn parse_ignore(input: &str) -> IResult<&str, &str> {
    alt((parse_generate_body, parse_uproperty, parse_ufunction)).parse(input)
}
#[cfg(test)]
mod tests {
    use crate::parser::cpp::class::{CppClass, CppParentClass, InheritanceVisibility};

    use crate::parser::cpp::ctype::CType::Path;
    use crate::parser::cpp::method::CppFunction;
    use crate::types::Parsable;
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
            let result = CppClass::parse(&input);

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
    fn test_ignore_template_specialisation() {
        let input = r#"template<>
        struct Test<int>
        {
        };"#;
        let result = CppClass::parse(&input);

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
        let result = CppClass::parse(&input);

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
    fn test_parse_struct_with_constructor() {
        let input = r#"struct Test
        {
            Test(){};
        };"#;
        let result = CppClass::parse(&input);

        assert_eq!(
            result,
            Ok((
                "",
                CppClass {
                    name: "Test",
                    methods: HashMap::from([
                        (
                            InheritanceVisibility::Private,
                            vec![CppFunction {
                                name: "Test",
                                ..Default::default()
                            }]
                        ),
                        (InheritanceVisibility::Protected, vec![]),
                        (InheritanceVisibility::Public, vec![]),
                    ]),
                    ..CppClass::default()
                }
            ))
        );
    }

    #[test]
    fn test_parse_empty_templated_struct() {
        let input = "template<typename T>\nstruct Test {};";
        let result = CppClass::parse(&input);

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
        let result = CppClass::parse(&input);

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
        let result = CppClass::parse(&input);

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
    fn test_parse_class_with_empty_constructor() {
        let input = r#"class test {
            public:
                test();
        };"#;
        let result = CppClass::parse(&input);

        assert_eq!(
            result,
            Ok((
                "",
                CppClass {
                    name: "test",
                    methods: HashMap::from([
                        (
                            InheritanceVisibility::Public,
                            vec![CppFunction {
                                name: "test",
                                ..Default::default()
                            }]
                        ),
                        (InheritanceVisibility::Protected, vec![]),
                        (InheritanceVisibility::Private, vec![]),
                    ]),
                    ..CppClass::default()
                }
            ))
        );
    }

    #[test]
    fn test_parse_class_with_inline_constructor() {
        let input = r#"class test {
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
        let result = CppClass::parse(&input);

        assert_eq!(
            result,
            Ok((
                "",
                CppClass {
                    name: "test",
                    methods: HashMap::from([
                        (
                            InheritanceVisibility::Public,
                            vec![CppFunction {
                                name: "test",
                                ..Default::default()
                            }]
                        ),
                        (InheritanceVisibility::Protected, vec![]),
                        (InheritanceVisibility::Private, vec![]),
                    ]),
                    ..CppClass::default()
                }
            ))
        );
    }

    #[test]
    fn test_parse_empty_cpp_with_api() {
        let input = format!("class MY_API test {{}};");
        let result = CppClass::parse(&input);

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
            let result = CppClass::parse(&input);

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
        let result = CppClass::parse(&input);

        assert_eq!(
            result,
            Ok((
                "",
                CppClass {
                    name: "test",
                    methods: HashMap::from([
                        (
                            InheritanceVisibility::Private,
                            vec![CppFunction {
                                name: "hello",
                                ..Default::default()
                            }]
                        ),
                        (InheritanceVisibility::Protected, vec![]),
                        (InheritanceVisibility::Public, vec![]),
                    ]),
                    ..CppClass::default()
                }
            ))
        );
    }

    #[test]
    fn test_parse_class_with_multiple_methods() {
        let input = "class test {void hello();\nvoid goodbye();};";
        let result = CppClass::parse(&input);

        assert_eq!(
            result,
            Ok((
                "",
                CppClass {
                    name: "test",
                    methods: HashMap::from([
                        (
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
                        ),
                        (InheritanceVisibility::Protected, vec![]),
                        (InheritanceVisibility::Public, vec![]),
                    ]),
                    ..CppClass::default()
                }
            ))
        );
    }
}

#[test]
fn test_parse_class_with_multiple_mixed_methods() {
    let input = format!("class test {{void hello();\nauto goodbye() -> int;}}");
    let result = CppClass::parse(&input);

    assert_eq!(
        result,
        Ok((
            "",
            CppClass {
                name: "test",
                methods: HashMap::from([
                    (
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
                    ),
                    (InheritanceVisibility::Protected, vec![]),
                    (InheritanceVisibility::Public, vec![]),
                ]),
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

    let result = CppClass::parse(input);

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
                members: HashMap::from([
                    (
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
                    ),
                    (InheritanceVisibility::Protected, vec![]),
                    (InheritanceVisibility::Public, vec![]),
                ]),
                ..CppClass::default()
            }
        ))
    );
}
