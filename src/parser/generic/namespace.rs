use crate::parser::generic::class::{Class, parse_class};
use crate::parser::generic::comment::parse_comment;
use crate::parser::generic::member::parse_member;
use crate::parser::generic::method::parse_method;
use crate::parser::{parse_str, ws};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{char, multispace0};
use nom::combinator::{map, opt};
use nom::multi::many_till;
use nom::sequence::preceded;
use nom::{IResult, Parser};
use nom_language::error::VerboseError;

pub trait Namespace<'a, ClassType>
where
    ClassType: Class<'a>,
{
    fn namespace(
        name: &'a str,
        namespaces: Vec<Self>,
        functions: Vec<ClassType::Method>,
        variables: Vec<ClassType::Member>,
        classes: Vec<ClassType>,
        comments: Vec<ClassType::Comment>,
    ) -> Self
    where
        Self: 'a + Sized;
}

pub fn parse_namespace<'a, NamespaceType, ClassType>(
    input: &'a str,
) -> IResult<&'a str, NamespaceType, VerboseError<&'a str>>
where
    NamespaceType: Namespace<'a, ClassType> + 'a,
    ClassType: Class<'a> + 'a,
{
    let (input, _) = tag("namespace")(input)?;
    let (input, name) = ws(parse_str).parse(input)?;
    let (input, _) = char('{')(input)?;

    let mut namespaces: Vec<NamespaceType> = Vec::new();
    let mut functions = Vec::new();
    let mut variables = Vec::new();
    let mut classes = Vec::new();
    let mut comments = Vec::new();

    let (input, (items, _)) =
        many_till(parse_namespace_item, preceded(multispace0, char('}'))).parse(input)?;
    for item in items {
        match item {
            NamespaceItem::Namespace(namespace) => namespaces.push(namespace),
            NamespaceItem::Class(class) => classes.push(class),
            NamespaceItem::Method(method) => functions.push(method),
            NamespaceItem::Variable(variable) => variables.push(variable),
            NamespaceItem::Comment(comment) => comments.push(comment),
            _ => {}
        }
    }

    Ok((
        input,
        NamespaceType::namespace(name, namespaces, functions, variables, classes, comments),
    ))
}

enum NamespaceItem<'a, NamespaceType, ClassType>
where
    NamespaceType: Namespace<'a, ClassType> + 'a,
    ClassType: Class<'a>,
{
    Ignore,
    Namespace(NamespaceType),
    Class(ClassType),
    Method(ClassType::Method),
    Variable(ClassType::Member),
    Comment(ClassType::Comment),
    End, // matched on `}` (+ optional `;`)
}

fn parse_namespace_item<'a, NamespaceType, ClassType>(
    input: &'a str,
) -> IResult<&'a str, NamespaceItem<'a, NamespaceType, ClassType>, VerboseError<&'a str>>
where
    NamespaceType: Namespace<'a, ClassType> + 'a,
    ClassType: Class<'a> + 'a,
{
    let (input, item) = preceded(
        multispace0,
        alt((
            map(char(';'), |_| NamespaceItem::Ignore),
            map(parse_namespace, NamespaceItem::Namespace),
            map(|i| parse_class(i, &vec![]), NamespaceItem::Class),
            map(parse_method, NamespaceItem::Method),
            map(parse_member, NamespaceItem::Variable),
            map(parse_comment, NamespaceItem::Comment),
            map(preceded(char('}'), opt(char(';'))), |_| NamespaceItem::End),
        )),
    )
    .parse(input)?;

    Ok((input, item))
}
