use crate::parser::cpp::ctype::{CType, parse_cpp_type};
use crate::parser::cpp::method::{parse_brace_block, parse_method_params, CppMethodParam};
use crate::parser::cpp::template::parse_template;
use crate::parser::generic::annotation::Annotation;
use crate::parser::generic::comment::parse_comment;
use crate::parser::ws;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_while1};
use nom::character::complete::{char, multispace0, multispace1};
use nom::combinator::{map, opt, recognize};
use nom::multi::{many0, separated_list0};
use nom::sequence::{delimited, preceded};
use nom::{IResult, Parser};
use nom_language::error::VerboseError;

pub trait Method<'a, AnnotationType, CommentType> {
    fn method(
        name: &'a str,
        return_type: Option<CType<'a>>,
        template_params: Vec<CType<'a>>,
        params: Vec<CppMethodParam<'a>>,
        storage_qualifiers: Vec<CppStorageQualifier>,
        post_param_qualifiers: Vec<PostParamQualifier>,
        special: Option<SpecialMember>,
        comment: Option<CommentType>,
        annotations: Vec<AnnotationType>,
    ) -> Self
    where
        AnnotationType: Annotation<'a>,
        CommentType: From<String>,
        Self: 'a;
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum CppStorageQualifier {
    Inline,
    Constexpr,
    Explicit,
    Friend,
    Static,
    Virtual,
}

impl From<&str> for CppStorageQualifier {
    fn from(value: &str) -> Self {
        match value {
            "inline" => CppStorageQualifier::Inline,
            "constexpr" => CppStorageQualifier::Constexpr,
            "explicit" => CppStorageQualifier::Explicit,
            "friend" => CppStorageQualifier::Friend,
            "static" => CppStorageQualifier::Static,
            "virtual" => CppStorageQualifier::Virtual,
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum PostParamQualifier {
    Const,
    Noexcept,
    Override,
    Final,
}

impl From<&str> for PostParamQualifier {
    fn from(value: &str) -> Self {
        match value {
            "const" => PostParamQualifier::Const,
            "noexcept" => PostParamQualifier::Noexcept,
            "override" => PostParamQualifier::Override,
            "final" => PostParamQualifier::Final,
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum SpecialMember {
    PureVirtual,
    Defaulted,
    Deleted,
}

fn special_member(input: &str) -> IResult<&str, SpecialMember, VerboseError<&str>> {
    preceded(
        ws(char('=')),
        alt((
            map(tag("0"), |_| SpecialMember::PureVirtual),
            map(tag("default"), |_| SpecialMember::Defaulted),
            map(tag("deleted"), |_| SpecialMember::Deleted),
        )),
    )
    .parse(input)
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
    let (input, storage_qualifiers) = opt(storage_qualifiers).parse(input)?;
    let (input, template_params) = opt(parse_template).parse(input)?;
    let (input, (return_type, name)) = alt((
        map(
            (parse_cpp_type, multispace1, method_name),
            |(ret, _, name)| (Some(ret), name),
        ),
        map(method_name, |name| (None, name)),
    ))
    .parse(input)?;

    let (input, params) = parse_method_params.parse(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = member_initializer_list.parse(input)?;
    let (input, return_type_trailing) = method_trailing_return.parse(input)?;
    let (input, post_param_qualifiers) = post_param_qualifiers(input)?;
    let (input, special) = opt(special_member).parse(input)?;
    let (input, _) = opt(preceded(multispace0, parse_brace_block)).parse(input)?;

    let return_type = match (return_type, return_type_trailing) {
        (None, None) => None,
        (None, Some(return_type_trailing)) => Some(return_type_trailing),
        (Some(return_type), None) => Some(return_type),
        (Some(return_type), Some(return_type_trailing)) => match return_type == CType::Auto {
            true => Some(return_type_trailing),
            false => unreachable!(),
        },
    };

    let return_type = match &return_type {
        Some(x) if *x == CType::Path(vec!["void"]) => None,
        _ => return_type,
    };

    Ok((
        input,
        MethodType::method(
            name,
            return_type,
            template_params.unwrap_or_default(),
            params,
            storage_qualifiers.unwrap_or_default(),
            post_param_qualifiers,
            special,
            comment,
            annotations.unwrap_or_default(),
        ),
    ))
}

fn method_name(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    alt((
        recognize((char('~'), method_identifier)),
        operator_name,
        method_identifier,
    ))
    .parse(input)
}

fn method_identifier(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '~')(input)
}

fn operator_name(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    recognize((
        tag("operator"),
        alt((
            alt((
                tag("()"),
                tag("[]"),
                tag("->"),
                tag("~"),
                tag("+"),
                tag("-"),
                tag("*"),
                tag("/"),
                tag("%"),
                tag("^"),
                tag("&"),
                tag("|"),
                tag("!"),
                tag("="),
                tag("<"),
                tag(">"),
                tag("++"),
                tag("--"),
            )),
            alt((
                tag("<<"),
                tag(">>"),
                tag("=="),
                tag("!="),
                tag("<="),
                tag(">="),
                tag("<=>"),
                tag("+="),
                tag("-="),
                tag("*="),
                tag("/="),
                tag("%="),
                tag("^="),
                tag("&="),
                tag("|="),
                tag(","),
                tag("new"),
                tag("delete"),
            )),
        )),
    ))
    .parse(input)
}

fn post_param_qualifiers(
    input: &str,
) -> IResult<&str, Vec<PostParamQualifier>, VerboseError<&str>> {
    many0(delimited(
        multispace0,
        alt((
            map(tag("const"), |_| PostParamQualifier::Const),
            map(tag("noexcept"), |_| PostParamQualifier::Noexcept),
            map(tag("override"), |_| PostParamQualifier::Override),
            map(tag("final"), |_| PostParamQualifier::Final),
        )),
        multispace0,
    ))
    .parse(input)
}

fn storage_qualifiers(input: &str) -> IResult<&str, Vec<CppStorageQualifier>, VerboseError<&str>> {
    many0(ws(alt((
        map(tag("inline"), |_| CppStorageQualifier::Inline),
        map(tag("constexpr"), |_| CppStorageQualifier::Constexpr),
        map(tag("explicit"), |_| CppStorageQualifier::Explicit),
        map(tag("friend"), |_| CppStorageQualifier::Friend),
        map(tag("static"), |_| CppStorageQualifier::Static),
        map(tag("virtual"), |_| CppStorageQualifier::Virtual),
    ))))
    .parse(input)
}

fn method_trailing_return(input: &str) -> IResult<&str, Option<CType>, VerboseError<&str>> {
    opt(preceded(
        delimited(multispace0, tag("->"), multispace0),
        parse_cpp_type,
    ))
    .parse(input)
}

fn member_initializer(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    map(
        take_while1(|c: char| c.is_alphanumeric() || c != ',' && c != '{' && c != '\n'),
        |s: &str| s.trim(),
    )
    .parse(input)
}
fn member_initializer_list(input: &str) -> IResult<&str, Vec<&str>, VerboseError<&str>> {
    opt(preceded(
        preceded(multispace0, tag(":")),
        separated_list0(ws(char(',')), member_initializer),
    ))
    .parse(input)
    .map(|(i, opt_list)| (i, opt_list.unwrap_or_default()))
}
