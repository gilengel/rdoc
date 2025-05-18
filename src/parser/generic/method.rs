use crate::parser::cpp::ctype::{CType, parse_cpp_type};
use crate::parser::cpp::method::{
    CppFunctionInheritance, CppMethodParam, parse_brace_block, parse_method_params,
};
use crate::parser::cpp::template::parse_template;
use crate::parser::generic::annotation::Annotation;
use crate::parser::generic::comment::parse_comment;
use crate::parser::{parse_type_str, parse_ws_str, ws};
use nom::bytes::complete::{tag, take_until};
use nom::character::complete::{char, multispace0};
use nom::combinator::opt;
use nom::multi::{many0, separated_list0, separated_list1};
use nom::sequence::preceded;
use nom::{IResult, Parser};
use nom_language::error::VerboseError;

pub trait Method<'a, AnnotationType, CommentType> {
    fn method(
        name: &'a str,
        return_type: Option<CType<'a>>,
        template_params: Vec<CType<'a>>,
        params: Vec<CppMethodParam<'a>>,
        inheritance_modifiers: Vec<CppFunctionInheritance>,
        is_const: bool,
        is_interface: bool,
        comment: Option<CommentType>,
        annotations: Vec<AnnotationType>
    ) -> Self
    where
        AnnotationType: Annotation<'a>,
        CommentType: From<String>,
        Self: 'a;
}

fn interface(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    let (input, _) = (multispace0, char('='), multispace0, char('0')).parse(input)?;

    Ok((input, ""))
}

pub fn parse_method<'a, MethodType, AnnotationType, CommentType>(
    input: &'a str,
) -> IResult<&'a str, MethodType, VerboseError<&'a str>>
where
    AnnotationType: Annotation<'a>,
    CommentType: From<String>,
    MethodType: 'a + Method<'a, AnnotationType, CommentType>,
{
    let (input, annotations) = opt(many0(|i| AnnotationType::parse(i))).parse(input)?;

    let (input, comment) = opt(parse_comment::<CommentType>).parse(input)?;
    let (input, _) = multispace0.parse(input)?;
    let (input, _) = opt(parse_template).parse(input)?;

    let (input, all_before_params) = take_until("(").parse(input)?;

    let (_, all_before_params) =
        separated_list1(tag(" "), parse_type_str).parse(all_before_params)?;
    let all_before_params = all_before_params
        .iter()
        .filter(|x| **x != "")
        .map(|x| *x)
        .collect::<Vec<&str>>();

    let mut return_type = None;
    let name: &str = all_before_params.last().unwrap();

    // we eiter have auto {method_name} or {return_type} {method_name} -> no constructor
    if all_before_params.len() > 1 {}

    let (mut input, params) = ws(parse_method_params).parse(input)?;

    if all_before_params.contains(&"auto") {
        let (i, ctype) = preceded(ws((tag("->"), multispace0)), parse_cpp_type).parse(input)?;
        input = i;

        return_type = Some(ctype);
    } else if all_before_params.len() > 1 {
        let ctype = all_before_params[all_before_params.len() - 2];
        let ctype = parse_cpp_type.parse(ctype)?.1;

        return_type = Some(ctype);
    }

    if return_type == Some(CType::Path(vec!["void"])) {
        return_type = None;
    }

    let (input, _) = multispace0(input)?;

    let (input, function_params) = separated_list0(tag(","), parse_ws_str).parse(input)?;

    let (input, is_interface) = opt(interface).parse(input)?;

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
        MethodType::method(
            name,
            return_type,
            vec![],
            params,
            inheritance_modifiers,
            is_const,
            is_interface.is_some(),
            comment,
            annotations.unwrap_or_default()
        ),
    ))
}
