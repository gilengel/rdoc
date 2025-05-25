use crate::parser::cpp::comment::CppComment;
use crate::parser::cpp::ctype::{CType, parse_cpp_type};
use crate::parser::generic::annotation::NoAnnotation;

use crate::parser::generic::method::{
    CppStorageQualifier, Method, PostParamQualifier, SpecialMember,
};
use crate::parser::{parse_str, parse_ws_str, ws};
use nom::branch::alt;
use nom::bytes::complete::take_till1;
use nom::character::complete::{char, multispace0, none_of};
use nom::combinator::{map, opt, peek, recognize};
use nom::multi::{many0, separated_list0};
use nom::sequence::{delimited, preceded, terminated};
use nom::{IResult, Parser};
use nom_language::error::VerboseError;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct CppFunction<'a> {
    pub name: &'a str,
    pub return_type: Option<CType<'a>>,
    pub template_params: Vec<CType<'a>>,
    pub params: Vec<CppMethodParam<'a>>,
    pub storage_qualifiers: Vec<CppStorageQualifier>,
    pub post_param_qualifiers: Vec<PostParamQualifier>,
    pub special: Option<SpecialMember>,
    pub comment: Option<CppComment>,
}

impl<'a> Method<'a> for CppFunction<'a> {
    type MethodAnnotation = NoAnnotation;
    type Comment = CppComment;

    fn method(
        name: &'a str,
        return_type: Option<CType<'a>>,
        template_params: Vec<CType<'a>>,
        params: Vec<CppMethodParam<'a>>,
        storage_qualifiers: Vec<CppStorageQualifier>,
        post_param_qualifiers: Vec<PostParamQualifier>,
        special: Option<SpecialMember>,
        comment: Option<CppComment>,
        _: Vec<NoAnnotation>,
    ) -> Self {
        CppFunction {
            name,
            return_type,
            template_params,
            params,
            storage_qualifiers,
            post_param_qualifiers,
            special,
            comment,
        }
    }
}

impl<'a> Default for CppFunction<'a> {
    fn default() -> Self {
        Self {
            name: "",
            return_type: None,
            template_params: vec![],
            params: vec![],
            storage_qualifiers: vec![],
            post_param_qualifiers: vec![],
            special: None,
            comment: None,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct CppMethodParam<'a> {
    pub name: Option<&'a str>,
    pub ctype: CType<'a>,
    pub default_value: Option<CType<'a>>,
}

fn parse_function_pointer_param(input: &str) -> IResult<&str, CppMethodParam, VerboseError<&str>> {
    let (input, return_type) = ws(parse_cpp_type).parse(input)?;
    let (input, _) = (multispace0, char('(')).parse(input)?;
    let (input, _) = (multispace0, char('*'), multispace0).parse(input)?;
    let (input, _) = terminated(parse_str, (multispace0, char(')'))).parse(input)?;

    let (input, params) = delimited(
        char('('),
        separated_list0(char(','), ws(parse_cpp_type)),
        char(')'),
    )
    .parse(input)?;

    Ok((
        input,
        CppMethodParam {
            name: None,
            ctype: CType::Function(Box::from(return_type), params),
            default_value: None,
        },
    ))
}

fn parse_simple_param(input: &str) -> IResult<&str, CppMethodParam, VerboseError<&str>> {
    let (input, _) = multispace0(input)?;
    let (input, ctype) = parse_cpp_type(input)?;
    let (input, name) = opt(parse_ws_str).parse(input)?;

    let (input, default_value) = opt(preceded(
        (multispace0, char('='), multispace0),
        parse_cpp_type,
    ))
    .parse(input)?;

    Ok((
        input,
        CppMethodParam {
            name,
            ctype,
            default_value,
        },
    ))
}

fn parse_cpp_method_param(input: &str) -> IResult<&str, CppMethodParam, VerboseError<&str>> {
    alt((parse_function_pointer_param, parse_simple_param)).parse(input)
}

pub fn parse_method_params(input: &str) -> IResult<&str, Vec<CppMethodParam>, VerboseError<&str>> {
    let (input, _) = (char('('), multispace0).parse(input)?;

    let (input, params) = alt((
        map(peek(char(')')), |_| Vec::new()),
        separated_list0(
            delimited(multispace0, char(','), multispace0),
            parse_cpp_method_param,
        ),
    ))
    .parse(input)?;

    let (input, _) = delimited(multispace0, char(')'), multispace0).parse(input)?;

    Ok((input, params))
}

fn parse_brace_inner(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    recognize(many0(alt((
        // Match and recurse into nested braces
        parse_brace_block,
        // Match any non-brace characters
        recognize(take_till1(|c| c == '{' || c == '}')),
        // Match single braces (not part of a block)
        recognize(none_of("{}")),
    ))))
    .parse(input)
}

/// Matches a balanced block like `{ ... }`, including nested ones.
pub fn parse_brace_block(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    map(
        recognize(delimited(char('{'), parse_brace_inner, char('}'))),
        |_| "",
    )
    .parse(input)
}

#[cfg(test)]
mod tests {
    use crate::parser::cpp::comment::CppComment;
    use crate::parser::cpp::ctype::CType;
    use crate::parser::cpp::ctype::CType::{Const, Function, Generic, Path, Pointer, Reference};
    use crate::parser::cpp::method::{CppFunction, CppMethodParam, parse_brace_block};
    use crate::parser::generic::method::CppStorageQualifier::Virtual;
    use crate::parser::generic::method::PostParamQualifier::Final;
    use crate::parser::generic::method::{PostParamQualifier, SpecialMember, parse_method};
    use nom::Err::Error;
    use nom_language::error::VerboseError;
    use nom_language::error::VerboseErrorKind::Char;

    #[test]
    fn test_empty_braces() {
        let input = "{}";
        assert_eq!(parse_brace_block(&input), Ok(("", "")));
    }

    #[test]
    fn test_only_braces() {
        let input = "{} CONTENT {}";
        assert_eq!(parse_brace_block(&input), Ok((" CONTENT {}", "")));
    }

    #[test]
    fn test_parse_nested_braces() {
        let input = r#"{
                    int j = 42;
                    for(int i = 0; i < j; i++)
                    {
                        std::cout << j << std::endl;
                    }
                }"#;
        assert_eq!(parse_brace_block(input), Ok(("", "")))
    }

    #[test]
    fn test_fails_for_invalid_argument_input() {
        let input = "void muu(#INVALID) = 0";
        let result = parse_method::<CppFunction>(input);

        assert_eq!(
            result,
            Err(Error(VerboseError {
                errors: vec![("#INVALID) = 0", Char(')'))]
            }))
        );
    }
    #[test]
    fn test_method_without_params() {
        let input = "void method()";
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_method_with_optional_param() {
        let input = "void method(const int i = 0)";
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    params: vec![CppMethodParam {
                        name: Some("i"),
                        ctype: Const(Box::from(Path(vec!["int"]))),
                        default_value: Some(Path(vec!["0"])),
                    }],
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_method_with_c_function_param() {
        let input = "void method(int (*f)(int, int))";
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    params: vec![CppMethodParam {
                        name: None,
                        ctype: CType::Function(
                            Box::from(CType::Path(vec!["int"])),
                            vec![CType::Path(vec!["int"]), CType::Path(vec!["int"])]
                        ),
                        default_value: None,
                    }],
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_method_with_std_function_param() {
        let input = "void method(const std::function<int(int, int)>& f)";
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    params: vec![CppMethodParam {
                        name: Some("f"),
                        ctype: Reference(Box::from(Const(Box::from(Generic(
                            Box::from(Path(vec!["std", "function"])),
                            vec![CType::Function(
                                Box::from(CType::Path(vec!["int"])),
                                vec![CType::Path(vec!["int"]), CType::Path(vec!["int"])]
                            )]
                        ))))),
                        default_value: None,
                    }],
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_interface_method() {
        let input = "void method() = 0";
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    special: Some(SpecialMember::PureVirtual),
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_inline_method_without_params() {
        let input = r#"void method(){
            int i=42;
        }"#;
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_inline_method_oneline_without_params() {
        let input = r#"// Say hello to everyone
            void sayHello(){ std::cout << "Hi" << std::endl; }"#;
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "sayHello",
                    comment: Some(CppComment {
                        comment: "Say hello to everyone".to_string()
                    }),
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_method_with_single_line_comment() {
        let input = r#"// does something
            void method()
        "#;
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    comment: Some(CppComment {
                        comment: "does something".to_string()
                    }),
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_method_with_multi_line_comment() {
        let input = r#"/**
             * does something
             *
             * @return nothing
             */
            void method()
        "#;

        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    comment: Some(CppComment {
                        comment: "does something\n@return nothing".to_string()
                    }),
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_method_with_virtual_modifier() {
        for storage_qualifier in ["virtual", "static"] {
            let input = format!("{} void method()", storage_qualifier);

            let result = parse_method(&input);

            assert_eq!(
                result,
                Ok((
                    "",
                    CppFunction {
                        name: "method",
                        storage_qualifiers: vec![storage_qualifier.into()],
                        ..Default::default()
                    }
                ))
            );
        }
    }

    #[test]
    fn test_method_with_inheritance_modifier() {
        for post_param_qualifier in ["override", "final"] {
            let input = format!("virtual void method() {}", post_param_qualifier);
            let result = parse_method(&input);

            assert_eq!(
                result,
                Ok((
                    "",
                    CppFunction {
                        name: "method",
                        storage_qualifiers: vec![Virtual],
                        post_param_qualifiers: vec![post_param_qualifier.into()],
                        ..Default::default()
                    }
                ))
            );
        }
    }

    #[test]
    fn test_method_with_param() {
        let input = "void method(int a) final";
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    params: vec![CppMethodParam {
                        name: Some("a"),
                        ctype: Path(vec!["int"]),
                        default_value: None
                    }],
                    post_param_qualifiers: vec![Final],
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_method_with_template_return_type() {
        let input = "TArray<int32> method()";
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Some(Generic(
                        Box::from(Path(vec!["TArray"])),
                        vec![Path(vec!["int32"])]
                    )),
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_method_with_reference_param() {
        let input = "void method(int& a) final";
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    params: vec![CppMethodParam {
                        name: Some("a"),
                        ctype: Reference(Box::from(Path(vec!["int"]))),
                        default_value: None
                    }],
                    post_param_qualifiers: vec![Final],
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_method_with_const_reference_param() {
        let input = "void method(const int& a) final";
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    params: vec![CppMethodParam {
                        name: Some("a"),
                        ctype: Reference(Box::from(Const(Box::from(Path(vec!["int"]))))),
                        default_value: None
                    }],
                    post_param_qualifiers: vec![Final],
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_method_with_pointer_param() {
        let input = "void method(int* a) final";
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    params: vec![CppMethodParam {
                        name: Some("a"),
                        ctype: Pointer(Box::from(Path(vec!["int"]))),
                        default_value: None
                    }],
                    post_param_qualifiers: vec![Final],
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_method_with_multiple_params() {
        let input = "void method(int& a, std::string b) final";
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    params: vec![
                        CppMethodParam {
                            name: Some("a"),
                            ctype: Reference(Box::from(Path(vec!["int"]))),
                            default_value: None
                        },
                        CppMethodParam {
                            name: Some("b"),
                            ctype: Path(vec!["std", "string"]),
                            default_value: None
                        }
                    ],
                    post_param_qualifiers: vec![Final],
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_method_with_template_param() {
        let input = "void method(TArray<int32> a) final";
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    params: vec![CppMethodParam {
                        name: Some("a"),
                        ctype: Generic(Box::from(Path(vec!["TArray"])), vec![Path(vec!["int32"])]),
                        default_value: None
                    }],
                    post_param_qualifiers: vec![Final],
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_const_method() {
        let input = "void method() const";
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    post_param_qualifiers: vec![PostParamQualifier::Const],
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_method_with_trailing_return_type() {
        let input = "auto method(int* a) -> int** final";
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Some(Pointer(Box::from(Pointer(Box::from(Path(vec!["int"])))))),
                    params: vec![CppMethodParam {
                        name: Some("a"),
                        ctype: Pointer(Box::from(Path(vec!["int"]))),
                        default_value: None
                    }],
                    post_param_qualifiers: vec![Final],
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_method_with_lambda_param() {
        let input = "auto method(std::function<int(int)>& lambda) -> int";
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Some(Path(vec!["int"])),
                    params: vec![CppMethodParam {
                        name: Some("lambda"),
                        ctype: Reference(Box::from(Generic(
                            Box::from(Path(vec!["std", "function"])),
                            vec![Function(
                                Box::from(Path(vec!["int"])),
                                vec![Path(vec!["int"])],
                            )],
                        ))),
                        default_value: None
                    }],
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_template_method() {
        let input = "template<typename T>T method()";
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    template_params: vec![Path(vec!["T"])],
                    return_type: Some(Path(vec!["T"])),
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_template_enable_if_method() {
        let input = "template<typename Integer, typename = std::enable_if_t<std::is_integral<Integer>::value>> void method(Integer a)";
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: None,
                    template_params: vec![Path(vec!["Integer"]), Path(vec![])],

                    params: vec![CppMethodParam {
                        name: Some("a"),
                        ctype: Path(vec!["Integer"]),
                        default_value: None
                    },],
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_method_with_unnamed_param() {
        let input = "void method(int)";
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: None,
                    params: vec![CppMethodParam {
                        name: None,
                        ctype: Path(vec!["int"]),
                        default_value: None
                    },],
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_empty_template_method() {
        let input = "template<> void method()";
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    ..Default::default()
                }
            ))
        );
    }
    #[test]

    fn test_multiple_templates_method() {
        let input = "template<typename T, class S>T method()";
        let result = parse_method(input);
        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    template_params: vec![Path(vec!["T"]), Path(vec!["S"])],
                    return_type: Some(Path(vec!["T"])),
                    ..Default::default()
                }
            ))
        );
    }
}
