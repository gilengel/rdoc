use crate::parser::cpp::ctype::{CType, parse_cpp_type};
use crate::parser::cpp::member::CppMemberModifier;
use crate::parser::generic::annotation::Annotation;
use crate::parser::generic::comment::parse_comment;
use crate::parser::parse_ws_str;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{char, multispace0};
use nom::combinator::{map, opt};
use nom::multi::many0;
use nom::sequence::{delimited, preceded};
use nom::{IResult, Parser};
use nom_language::error::VerboseError;

pub enum MemberModifier {
    Static,
    Const,
    Inline,
}
pub trait Member<'a, AnnotationType, CommentType> {
    fn member(
        name: &'a str,
        ctype: CType<'a>,
        default_value: Option<CType<'a>>,
        comment: Option<CommentType>,
        modifiers: Vec<CppMemberModifier>,
        annotations: Vec<AnnotationType>,
    ) -> Self
    where
        AnnotationType: Annotation<'a>,
        Self: 'a;
}
pub fn parse_member<'a, MemberType, AnnotationType, CommentType>(
    input: &'a str,
) -> IResult<&'a str, MemberType, VerboseError<&'a str>>
where
    MemberType: 'a + Member<'a, AnnotationType, CommentType>,
    CommentType: From<String>,
    AnnotationType: Annotation<'a>,
{
    let (input, annotations) = opt(many0(|i| AnnotationType::parse(i))).parse(input)?;

    let (input, comment) = opt(parse_comment::<CommentType>).parse(input)?;
    let (input, _) = multispace0.parse(input)?;
    let (input, modifiers) = parse_modifiers(input)?;
    let (input, _) = multispace0.parse(input)?;
    let (input, ctype) = parse_cpp_type(input)?;
    let (input, name) = parse_ws_str(input)?;
    let (input, _) = multispace0.parse(input)?;

    let (input, default_value) = opt(alt((
        delimited(
            char('{'),
            delimited(multispace0, parse_cpp_type, multispace0),
            char('}'),
        ),
        preceded(
            char('='),
            delimited(multispace0, parse_cpp_type, multispace0),
        ),
    )))
    .parse(input)?;

    Ok((
        input,
        MemberType::member(name, ctype, default_value, comment, modifiers, annotations.unwrap_or_default()),
    ))
}

fn parse_modifier(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    preceded(
        multispace0,
        alt((tag("static"), tag("const"), tag("inline"))),
    )
    .parse(input)
}

fn parse_modifiers(input: &str) -> IResult<&str, Vec<CppMemberModifier>, VerboseError<&str>> {
    many0(map(parse_modifier, |x| CppMemberModifier::from(x))).parse(input)
}
