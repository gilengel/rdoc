use crate::parser::cpp::ctype::CType::Path;
use crate::parser::cpp::ctype::{CType, parse_cpp_type};
use crate::parser::cpp::{parse_ws_str, ws};
use nom::branch::alt;
use nom::character::complete::{char, multispace0};
use nom::combinator::{map, opt, peek};
use nom::multi::separated_list0;
use nom::sequence::delimited;
use nom::{IResult, Parser, bytes::complete::tag};
use crate::parser::cpp::comment::{parse_cpp_comment, CppComment};

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum CppFunctionInheritance {
    Virtual,
    Override,
    Final,
}

impl From<&str> for CppFunctionInheritance {
    fn from(input: &str) -> Self {
        match input {
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
    pub return_type: CType<'a>,
    pub template_params: Vec<CType<'a>>,
    pub params: Vec<CppMethodParam<'a>>,
    pub inheritance_modifiers: Vec<CppFunctionInheritance>,
    pub is_const: bool,
    pub comment: Option<CppComment>
}

impl<'a> Default for CppFunction<'a> {
    fn default() -> Self {
        Self {
            name: "",
            return_type: Path(vec!["void"]),
            template_params: vec![],
            params: vec![],
            inheritance_modifiers: vec![],
            is_const: false,
            comment: None
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct CppMethodParam<'a> {
    name: Option<&'a str>,
    ctype: CType<'a>,
    is_const: bool,
}

fn parse_template_param(input: &str) -> IResult<&str, CType> {
    let (input, _) = ws(alt((tag("typename"), tag("class")))).parse(input)?;
    let (input, ctype) = parse_cpp_type(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = opt((ws(char('=')), multispace0, parse_cpp_type)).parse(input)?;

    Ok((input, ctype))
}
fn parse_template(input: &str) -> IResult<&str, Vec<CType>> {
    let (input, _) = ws(tag("template")).parse(input)?;
    let (input, _) = char('<').parse(input)?;
    let (input, params) = separated_list0(tag(","), parse_template_param).parse(input)?;
    let (input, _) = ws(char('>')).parse(input)?;

    Ok((input, params))
}
fn parse_cpp_method_param(input: &str) -> IResult<&str, CppMethodParam> {
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

pub fn parse_method_params(input: &str) -> IResult<&str, Vec<CppMethodParam>> {
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

pub fn parse_classic_method(input: &str) -> IResult<&str, (&str, CType, Vec<CppMethodParam>)> {
    let (input, return_type) = parse_cpp_type(input)?;
    let (input, name) = parse_ws_str(input)?;
    let (input, params) = ws(parse_method_params).parse(input)?;

    Ok((input, (name, return_type, params)))
}
pub fn parse_trailing_return_method(
    input: &str,
) -> IResult<&str, (&str, CType, Vec<CppMethodParam>)> {
    let (input, _) = tag("auto").parse(input)?;
    let (input, _) = multispace0(input)?;
    let (input, name) = parse_ws_str(input)?;
    let (input, params) = ws(parse_method_params).parse(input)?;
    let (input, _) = tag("->").parse(input)?;
    let (input, return_type) = ws(parse_cpp_type).parse(input)?;

    Ok((input, (name, return_type, params)))
}
pub fn parse_cpp_method(input: &str) -> IResult<&str, CppFunction> {
    let (input, _) = multispace0.parse(input)?;
    let (input, comment) = opt(parse_cpp_comment).parse(input)?;
    let (input, _) = opt(parse_template).parse(input)?;
    let (input, is_virtual) = opt(tag("virtual")).parse(input)?;
    let (input, _) = multispace0(input)?;

    let (input, function) =
        alt((parse_trailing_return_method, parse_classic_method)).parse(input)?;

    let (input, function_params) = separated_list0(tag(","), parse_ws_str).parse(input)?;
    //let (input, _) = char(';').parse(input)?;
    //let (input, _) = multispace0.parse(input)?;


    let mut inheritance_modifiers = Vec::new();

    if is_virtual.is_some() {
        inheritance_modifiers.push(CppFunctionInheritance::Virtual);
    }

    if function_params.contains(&"override") {
        inheritance_modifiers.push(CppFunctionInheritance::Override);
    }

    if function_params.contains(&"final") {
        inheritance_modifiers.push(CppFunctionInheritance::Final);
    }

    let is_const = function_params.contains(&"const");

    Ok((
        input,
        CppFunction {
            name: function.0,
            return_type: function.1,
            params: function.2,
            inheritance_modifiers,
            is_const,
            template_params: vec![],
            comment
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::parser::cpp::comment::CppComment;
    use crate::parser::cpp::ctype::CType::{Function, Generic, Path, Pointer, Reference};
    use crate::parser::cpp::method::CppFunctionInheritance::{Final, Virtual};
    use crate::parser::cpp::method::{
        CppFunction, CppFunctionInheritance, CppMethodParam, parse_cpp_method,
    };

    #[test]
    fn test_method_without_params() {
        let input = "void method()";
        assert_eq!(
            parse_cpp_method(&input[..]),
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
    fn test_method_with_single_line_comment() {
        let input = r#"
            // does something
            void method()
        "#;

        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    comment: Some(CppComment { comment: "does something".to_string() }),
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_method_with_multi_line_comment() {
        let input = r#"
            /**
             * does something
             *
             * @return nothing
             */
            void method()
        "#;

        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    comment: Some(CppComment { comment: "does something\n@return nothing".to_string() }),
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_method_with_virtual_modifier() {
        for inheritance_modifier in ["", "virtual"] {
            let input = format!("{} void method()", inheritance_modifier);

            let inheritance_modifiers = match inheritance_modifier {
                "virtual" => vec![Virtual],
                _ => vec![],
            };

            assert_eq!(
                parse_cpp_method(&input[..]),
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
            assert_eq!(
                parse_cpp_method(&input[..]),
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
        assert_eq!(
            parse_cpp_method(&input[..]),
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
        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Generic(
                        Box::from(Path(vec!["TArray"])),
                        vec![Path(vec!["int32"])]
                    ),
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_method_with_reference_param() {
        let input = "void method(int& a) final";
        assert_eq!(
            parse_cpp_method(&input[..]),
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
        assert_eq!(
            parse_cpp_method(&input[..]),
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
        assert_eq!(
            parse_cpp_method(&input[..]),
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
        assert_eq!(
            parse_cpp_method(&input[..]),
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
        assert_eq!(
            parse_cpp_method(&input[..]),
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
        assert_eq!(
            parse_cpp_method(&input[..]),
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
        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Pointer(Box::from(Pointer(Box::from(Path(vec!["int"]))))),
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

        let param_ctype = Reference(Box::from(Generic(
            Box::from(Path(vec!["std", "function"])),
            vec![Function(
                Box::from(Path(vec!["int"])),
                vec![Path(vec!["int"])],
            )],
        )));
        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Path(vec!["int"]),
                    params: vec![CppMethodParam {
                        name: Some("lambda"),
                        ctype: param_ctype,
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
        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Path(vec!["T"]),
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_template_enable_if_method() {
        let input = "template<typename Integer, typename = std::enable_if_t<std::is_integral<Integer>::value>> void method(Integer a)";
        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Path(vec!["void"]),
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
        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Path(vec!["void"]),
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
        assert_eq!(
            parse_cpp_method(&input[..]),
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
        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Path(vec!["T"]),
                    ..Default::default()
                }
            ))
        );
    }
}
