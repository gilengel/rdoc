use crate::parser::cpp::comment::CppComment;
use crate::parser::cpp::ctype::{CType, parse_cpp_type};
use crate::parser::generic::annotation::NoAnnotation;

use crate::parser::generic::method::Method;
use crate::parser::parse_ws_str;
use nom::branch::alt;
use nom::bytes::complete::take_till1;
use nom::character::complete::{char, multispace0, none_of};
use nom::combinator::{map, opt, peek, recognize};
use nom::multi::{many0, separated_list0};
use nom::sequence::delimited;
use nom::{IResult, Parser, bytes::complete::tag};
use nom_language::error::VerboseError;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum CppFunctionInheritance {
    Static,
    Virtual,
    Override,
    Final,
}

impl From<&str> for CppFunctionInheritance {
    fn from(input: &str) -> Self {
        match input {
            "static" => CppFunctionInheritance::Static,
            "virtual" => CppFunctionInheritance::Virtual,
            "override" => CppFunctionInheritance::Override,
            "final" => CppFunctionInheritance::Final,
            _ => unimplemented!(),
        }
    }
}
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct CppFunction<'a> {
    pub name: &'a str,
    pub return_type: Option<CType<'a>>,
    pub template_params: Vec<CType<'a>>,
    pub params: Vec<CppMethodParam<'a>>,
    pub inheritance_modifiers: Vec<CppFunctionInheritance>,
    pub is_const: bool,
    pub is_interface: bool,
    pub comment: Option<CppComment>,
}

impl<'a> Method<'a, NoAnnotation, CppComment> for CppFunction<'a> {
    fn method(
        name: &'a str,
        return_type: Option<CType<'a>>,
        template_params: Vec<CType<'a>>,
        params: Vec<CppMethodParam<'a>>,
        inheritance_modifiers: Vec<CppFunctionInheritance>,
        is_const: bool,
        is_interface: bool,
        comment: Option<CppComment>,
        _annotations: Vec<NoAnnotation>,
    ) -> Self {
        CppFunction {
            name,
            return_type,
            template_params,
            params,
            inheritance_modifiers,
            is_const,
            is_interface,
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
            inheritance_modifiers: vec![],
            is_const: false,
            is_interface: false,
            comment: None,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct CppMethodParam<'a> {
    name: Option<&'a str>,
    ctype: CType<'a>,
    is_const: bool,
}

fn parse_cpp_method_param(input: &str) -> IResult<&str, CppMethodParam, VerboseError<&str>> {
    let (input, is_const) = opt(tag("const")).parse(input)?;
    let (input, _) = multispace0(input)?;
    let (input, ctype) = parse_cpp_type(input)?;
    let (input, name) = opt(parse_ws_str).parse(input)?;

    Ok((
        input,
        CppMethodParam {
            name,
            ctype,
            is_const: is_const.is_some(),
        },
    ))
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
    use crate::parser::cpp::ctype::CType::{Function, Generic, Path, Pointer, Reference};
    use crate::parser::cpp::method::CppFunctionInheritance::{Final, Static, Virtual};
    use crate::parser::cpp::method::{
        CppFunction, CppFunctionInheritance, CppMethodParam, parse_brace_block,
    };
    use crate::parser::generic::method::parse_method;

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
    fn test_interface_method() {
        let input = "void method() = 0";
        let result = parse_method(input);

        assert_eq!(
            result,
            Ok((
                "",
                CppFunction {
                    name: "method",
                    is_interface: true,
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
        for inheritance_modifier in ["", "virtual", "static"] {
            let input = format!("{} void method()", inheritance_modifier);
            let inheritance_modifiers = match inheritance_modifier {
                "virtual" => vec![Virtual],
                "static" => vec![Static],
                _ => vec![],
            };

            let result = parse_method(&input);

            assert_eq!(
                result,
                Ok((
                    "",
                    CppFunction {
                        name: "method",
                        inheritance_modifiers,
                        ..Default::default()
                    }
                ))
            );
        }
    }

    #[test]
    fn test_method_with_inheritance_modifier() {
        for inheritance_modifier in ["override", "final"] {
            let input = format!("virtual void method() {}", inheritance_modifier);
            let result = parse_method(&input);

            assert_eq!(
                result,
                Ok((
                    "",
                    CppFunction {
                        name: "method",
                        inheritance_modifiers: vec![
                            Virtual,
                            CppFunctionInheritance::from(inheritance_modifier)
                        ],
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
                        is_const: false,
                        ctype: Path(vec!["int"])
                    }],
                    inheritance_modifiers: vec![Final],
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
                        is_const: false,
                        ctype: Reference(Box::from(Path(vec!["int"])))
                    }],
                    inheritance_modifiers: vec![Final],
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
                        is_const: true,
                        ctype: Reference(Box::from(Path(vec!["int"])))
                    }],
                    inheritance_modifiers: vec![Final],
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
                        is_const: false,
                        ctype: Pointer(Box::from(Path(vec!["int"])))
                    }],
                    inheritance_modifiers: vec![Final],
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
                            is_const: false,
                            ctype: Reference(Box::from(Path(vec!["int"])))
                        },
                        CppMethodParam {
                            name: Some("b"),
                            is_const: false,
                            ctype: Path(vec!["std", "string"])
                        }
                    ],
                    inheritance_modifiers: vec![Final],
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
                        is_const: false,
                        ctype: Generic(Box::from(Path(vec!["TArray"])), vec![Path(vec!["int32"])])
                    }],
                    inheritance_modifiers: vec![Final],
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
                    is_const: true,
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
                        is_const: false,
                        ctype: Pointer(Box::from(Path(vec!["int"])))
                    }],
                    inheritance_modifiers: vec![Final],
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
                        is_const: false
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
                    params: vec![CppMethodParam {
                        name: Some("a"),
                        ctype: Path(vec!["Integer"]),
                        is_const: false
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
                        is_const: false
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
                    return_type: Some(Path(vec!["T"])),
                    ..Default::default()
                }
            ))
        );
    }
}
