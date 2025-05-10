use crate::parser::cpp::ctype::{CType, ctype};
use crate::parser::cpp::parse_ws_str;
use nom::character::complete::multispace0;
use nom::combinator::opt;
use nom::multi::separated_list0;
use nom::{IResult, Parser, bytes::complete::tag};

#[derive(Debug, Eq, PartialEq, Clone)]
enum CppFunctionInheritance {
    Virtual,
    Override,
    Final,
}
#[derive(Debug, Eq, PartialEq, Clone)]
struct CppFunction<'a> {
    name: &'a str,
    return_type: CType<'a>,
    params: Vec<CppMethodParam<'a>>,
    inheritance_modifiers: Vec<CppFunctionInheritance>,
    is_const: bool,
}

#[derive(Debug, Eq, PartialEq, Clone)]
struct CppMethodParam<'a> {
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

fn parse_cpp_method(input: &str) -> IResult<&str, CppFunction> {
    let (input, is_virtual) = opt(tag("virtual")).parse(input)?;
    let (input, _) = multispace0(input)?;
    let (input, return_type) = ctype(input)?;
    let (input, name) = parse_ws_str(input)?;
    let (input, _) = tag("(").parse(input)?;
    let (input, params) = separated_list0(tag(","), parse_cpp_method_param).parse(input)?;
    let (input, _) = tag(")").parse(input)?;

    let (input, function_params) = separated_list0(tag(" "), parse_ws_str).parse(input)?;

    let mut inheritance_modifiers = Vec::new();
    if function_params.contains(&"override") {
        inheritance_modifiers.push(CppFunctionInheritance::Override);
    }

    if function_params.contains(&"final") {
        inheritance_modifiers.push(CppFunctionInheritance::Final);
    }

    let is_const = function_params.contains(&"const");

    if is_virtual.is_some() {
        inheritance_modifiers.push(CppFunctionInheritance::Virtual);
    }

    Ok((
        input,
        CppFunction {
            name,
            return_type,
            params,
            inheritance_modifiers,
            is_const,
        },
    ))
}

#[cfg(test)]
mod tests {
    use crate::parser::cpp::ctype::CType::{Base, Generic, Pointer, Reference};
    use crate::parser::cpp::method::CppFunctionInheritance::{Final, Override, Virtual};
    use crate::parser::cpp::method::{CppFunction, CppMethodParam, parse_cpp_method};

    #[test]
    fn test_method_without_params() {
        let input = "void method()";
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
        let input = "virtual void method()";
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
    fn test_method_with_overrides() {
        let input = "virtual void method() override";
        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Base("void"),
                    params: vec![],
                    inheritance_modifiers: vec![Virtual, Override],
                    is_const: false,
                }
            ))
        );
    }

    #[test]
    fn test_method_with_final() {
        let input = "void method() final";
        assert_eq!(
            parse_cpp_method(&input[..]),
            Ok((
                "",
                CppFunction {
                    name: "method",
                    return_type: Base("void"),
                    params: vec![],
                    inheritance_modifiers: vec![Final],
                    is_const: false,
                }
            ))
        );
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
        let input = "TArray<int32> method()";
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
        let input = "void method(int& a) final";
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
        let input = "void method(const int& a) final";
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
        let input = "void method(int* a) final";
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
        let input = "void method(int& a, std::string b) final";
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
        let input = "void method(TArray<int32> a) final";
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
        let input = "void method() const";
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
}
