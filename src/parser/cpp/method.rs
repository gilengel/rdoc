use crate::parser::cpp::comment::{CppComment, parse_cpp_comment};
use crate::parser::cpp::ctype::{CType, parse_cpp_type};
use crate::parser::cpp::{parse_type_str, parse_ws_str, ws};
use nom::branch::alt;
use nom::character::complete::{char, multispace0};
use nom::combinator::{map, opt, peek};
use nom::multi::{separated_list0, separated_list1};
use nom::sequence::{delimited, preceded};
use nom::{IResult, Parser, bytes::complete::tag};
use nom::bytes::complete::take_until;
use crate::parser::cpp::template::parse_template;

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

/// Parses a balanced block of `{...}` including nested ones.
pub fn parse_brace_block(input: &str) -> IResult<&str, &str> {
    let (input, _) = multispace0(input)?;
    let (input, _) = char('{')(input)?;
    let (input, _) = take_until("{").or(take_until("}")).parse(input)?;

    // parse nested braces
    let (input, _) = opt(parse_brace_block).parse(input)?;

    let (input, _) = multispace0(input)?;
    let (input, _) = char('}')(input)?;

    Ok((input, ""))
}
pub fn parse_cpp_method(input: &str) -> IResult<&str, CppFunction> {
    let (input, _) = multispace0.parse(input)?;
    let (input, comment) = opt(parse_cpp_comment).parse(input)?;
    let (input, _) = multispace0.parse(input)?;
    let (input, _) = opt(parse_template).parse(input)?;

    let (input, all_before_params) = take_until("(").parse(input)?;
    let (_, all_before_params) = separated_list1(tag(" "), parse_type_str).parse(all_before_params)?;
    let all_before_params = all_before_params.iter().filter(|x| **x != "").map(|x| *x).collect::<Vec<&str>>();

    let mut return_type = None;
    let name : &str = all_before_params.last().unwrap();

    // we eiter have auto {method_name} or {return_type} {method_name} -> no constructor
    if all_before_params.len() > 1 {

    }


    let (mut input, params) = ws(parse_method_params).parse(input)?;

    if all_before_params.contains(&"auto") {
        let (i, ctype) = preceded(ws((tag("->"), multispace0)), parse_cpp_type).parse(input)?;
        input = i;

        return_type = Some(ctype);
    }else if all_before_params.len() > 1 {
        let ctype = all_before_params[all_before_params.len()-2];
        let ctype = parse_cpp_type.parse(ctype)?.1;


        return_type = Some(ctype);
    }

    if return_type == Some(CType::Path(vec!["void"]))
    {
        return_type = None;
    }

    let (input, _) = multispace0(input)?;


    let (input, function_params) = separated_list0(tag(","), parse_ws_str).parse(input)?;

    let (input, is_interface) = opt((multispace0, char('='), multispace0, char('0'))).parse(input)?;

    let (input, _) = opt(parse_brace_block).parse(input)?;


    let mut inheritance_modifiers = Vec::new();

    if all_before_params.contains(&"static") {
        inheritance_modifiers.push(CppFunctionInheritance::Static);
    }

    if all_before_params.contains(&"virtual") {
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
            name,
            return_type,
            params,
            inheritance_modifiers,
            is_const,
            is_interface: is_interface.is_some(),
            template_params: vec![],
            comment,
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::parser::cpp::comment::CppComment;
    use crate::parser::cpp::ctype::CType::{Function, Generic, Path, Pointer, Reference};
    use crate::parser::cpp::method::CppFunctionInheritance::{Final, Static, Virtual};
    use crate::parser::cpp::method::{CppFunction, CppFunctionInheritance, CppMethodParam, parse_cpp_method, parse_brace_block};

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
    fn test_interface_method() {
        let input = "void method() = 0";
        assert_eq!(
            parse_cpp_method(&input[..]),
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
    fn test_inline_method_oneline_without_params() {
        let input = r#"            // Say hello to everyone
            void sayHello(){ std::cout << "Hi" << std::endl; }"#;

        assert_eq!(
            parse_cpp_method(input),
            Ok((
                "",
                CppFunction {
                    name: "sayHello",
                    comment: Some(CppComment{comment: "Say hello to everyone".to_string()}),
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
            parse_cpp_method(input),
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
                    return_type: Some(Path(vec!["int"])),
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
                    return_type: Some(Path(vec!["T"])),
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
                    return_type: None,
                    params: vec![CppMethodParam {
                        name: Some("a"),
                        ctype: Path(vec!["Integer"]),
                        is_const: false
                    }, ],
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
                    return_type: None,
                    params: vec![CppMethodParam {
                        name: None,
                        ctype: Path(vec!["int"]),
                        is_const: false
                    }, ],
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
                    return_type: Some(Path(vec!["T"])),
                    ..Default::default()
                }
            ))
        );
    }
}
