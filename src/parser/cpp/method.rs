use crate::parser::cpp::ctype::{CType, ctype};
use crate::parser::cpp::{parse_ws_str, ws};
use nom::branch::alt;
use nom::character::complete::multispace0;
use nom::combinator::opt;
use nom::multi::separated_list0;
use nom::{IResult, Parser, bytes::complete::tag};

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
    pub params: Vec<CppMethodParam<'a>>,
    pub inheritance_modifiers: Vec<CppFunctionInheritance>,
    pub is_const: bool,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct CppMethodParam<'a> {
    name: &'a str,
    ctype: CType<'a>,
    is_const: bool,
}

fn parse_cpp_method_param(input: &str) -> IResult<&str, CppMethodParam> {
    let (input, is_const) = opt(tag("const")).parse(input)?;
    let (input, _) = multispace0(input)?;
    let (input, ctype) = ctype(input)?;
    let (input, name) = parse_ws_str(input)?;

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
    let (input, _) = tag("(").parse(input)?;
    let (input, params) = separated_list0(tag(","), parse_cpp_method_param).parse(input)?;
    let (input, _) = tag(")").parse(input)?;

    Ok((input, params))
}

pub fn parse_classic_method(input: &str) -> IResult<&str, (&str, CType, Vec<CppMethodParam>)> {
    let (input, return_type) = ctype(input)?;
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
    let (input, return_type) = ws(ctype).parse(input)?;

    Ok((input, (name, return_type, params)))
}
pub fn parse_cpp_method(input: &str) -> IResult<&str, CppFunction> {
    let (input, is_virtual) = opt(tag("virtual")).parse(input)?;
    let (input, _) = multispace0(input)?;

    let (input, function) =
        alt((parse_trailing_return_method, parse_classic_method)).parse(input)?;

    let (input, function_params) = separated_list0(tag(" "), parse_ws_str).parse(input)?;
    let (input, _) = tag(";").parse(input)?;

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
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::parser::cpp::ctype::CType::{Base, Generic, Pointer, Reference};
    use crate::parser::cpp::method::CppFunctionInheritance::{Final, Virtual};
    use crate::parser::cpp::method::{CppFunction, CppMethodParam, parse_cpp_method, CppFunctionInheritance};

    #[test]
    fn test_method_without_params() {
        let input = "void method();";
        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Base("void"),
                    params: vec![],
                    inheritance_modifiers: vec![],
                    is_const: false,
                }
            ))
        );
    }

    #[test]
    fn test_method_with_virtual() {
        let input = "virtual void method();";
        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Base("void"),
                    params: vec![],
                    inheritance_modifiers: vec![Virtual],
                    is_const: false,
                }
            ))
        );
    }

    #[test]
    fn test_method_with_inheritance_modifier() {
        for inheritance_modifier in ["override", "final"] {
            let input = format!("virtual void method() {};", inheritance_modifier);
            assert_eq!(
                parse_cpp_method(&input[..]),
                Ok((
                    "",
                    CppFunction {
                        name: "method",
                        return_type: Base("void"),
                        params: vec![],
                        inheritance_modifiers: vec![Virtual, CppFunctionInheritance::from(inheritance_modifier)],
                        is_const: false,
                    }
                ))
            );
        }

    }

    #[test]
    fn test_method_with_param() {
        let input = "void method(int a) final;";
        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Base("void"),
                    params: vec![CppMethodParam {
                        name: "a",
                        is_const: false,
                        ctype: Base("int")
                    }],
                    inheritance_modifiers: vec![Final],
                    is_const: false,
                }
            ))
        );
    }

    #[test]
    fn test_method_with_template_return_type() {
        let input = "TArray<int32> method();";
        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Generic("TArray", Box::from(Base("int32"))),
                    params: vec![],
                    inheritance_modifiers: vec![],
                    is_const: false,
                }
            ))
        );
    }

    #[test]
    fn test_method_with_reference_param() {
        let input = "void method(int& a) final;";
        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Base("void"),
                    params: vec![CppMethodParam {
                        name: "a",
                        is_const: false,
                        ctype: Reference(Box::from(Base("int")))
                    }],
                    inheritance_modifiers: vec![Final],
                    is_const: false,
                }
            ))
        );
    }

    #[test]
    fn test_method_with_const_reference_param() {
        let input = "void method(const int& a) final;";
        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Base("void"),
                    params: vec![CppMethodParam {
                        name: "a",
                        is_const: true,
                        ctype: Reference(Box::from(Base("int")))
                    }],
                    inheritance_modifiers: vec![Final],
                    is_const: false,
                }
            ))
        );
    }

    #[test]
    fn test_method_with_pointer_param() {
        let input = "void method(int* a) final;";
        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Base("void"),
                    params: vec![CppMethodParam {
                        name: "a",
                        is_const: false,
                        ctype: Pointer(Box::from(Base("int")))
                    }],
                    inheritance_modifiers: vec![Final],
                    is_const: false,
                }
            ))
        );
    }

    #[test]
    fn test_method_with_multiple_params() {
        let input = "void method(int& a, std::string b) final;";
        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Base("void"),
                    params: vec![
                        CppMethodParam {
                            name: "a",
                            is_const: false,
                            ctype: Reference(Box::from(Base("int")))
                        },
                        CppMethodParam {
                            name: "b",
                            is_const: false,
                            ctype: Base("std::string")
                        }
                    ],
                    inheritance_modifiers: vec![Final],
                    is_const: false,
                }
            ))
        );
    }

    #[test]
    fn test_method_with_template_param() {
        let input = "void method(TArray<int32> a) final;";
        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Base("void"),
                    params: vec![CppMethodParam {
                        name: "a",
                        is_const: false,
                        ctype: Generic("TArray", Box::from(Base("int32")))
                    }],
                    inheritance_modifiers: vec![Final],
                    is_const: false,
                }
            ))
        );
    }

    #[test]
    fn test_const_method() {
        let input = "void method() const;";
        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Base("void"),
                    params: vec![],
                    inheritance_modifiers: vec![],
                    is_const: true,
                }
            ))
        );
    }

    #[test]
    fn test_method_with_trailing_return_type() {
        let input = "auto method(int* a) -> int** final;";
        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Pointer(Box::from(Pointer(Box::from(Base("int"))))),
                    params: vec![CppMethodParam {
                        name: "a",
                        is_const: false,
                        ctype: Pointer(Box::from(Base("int")))
                    }],
                    inheritance_modifiers: vec![Final],
                    is_const: false,
                }
            ))
        );
    }
}
