use crate::parser::cpp::method::{Method, parse_method};
use crate::parser::generic::class::{Class, parse_class};
use crate::parser::generic::comment::parse_comment;
use crate::parser::generic::member::{Member, parse_member};
use crate::parser::{parse_str, ws};
use nom::{IResult, Parser};
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{char, multispace0};
use nom::combinator::{map, opt};
use nom::multi::many_till;
use nom::sequence::preceded;
use nom_language::error::VerboseError;

pub trait Namespace<'a, ClassType, VariableType, MethodType, CommentType>
where
    ClassType: Class<'a, VariableType, MethodType, CommentType>,
    MethodType: Method<'a, CommentType>,
    VariableType: Member<'a, CommentType>,
    CommentType: From<String>,
{
    fn namespace(
        name: &'a str,
        namespaces: Vec<Self>,
        functions: Vec<MethodType>,
        variables: Vec<VariableType>,
        classes: Vec<ClassType>,
        comments: Vec<CommentType>,
    ) -> Self
    where
        Self: 'a + Sized;
}

pub fn parse_namespace<'a, NamespaceType, ClassType, VariableType, MethodType, CommentType>(
    input: &'a str,
) -> IResult<&'a str, NamespaceType, VerboseError<&'a str>>
where
    NamespaceType: Namespace<'a, ClassType, VariableType, MethodType, CommentType> + 'a,
    ClassType: Class<'a, VariableType, MethodType, CommentType> + 'a,
    MethodType: Method<'a, CommentType> + 'a,
    VariableType: Member<'a, CommentType> + 'a,
    CommentType: From<String> + 'a,
{
    let (input, _) = tag("namespace")(input)?;
    let (input, name) = ws(parse_str).parse(input)?;
    let (input, _) = char('{')(input)?;

    let mut namespaces: Vec<NamespaceType> = Vec::new();
    let mut functions = Vec::new();
    let mut variables = Vec::new();
    let mut classes = Vec::new();
    let mut comments = Vec::new();

    let (input, (items, _)) = many_till(
        parse_namespace_item,
        preceded(multispace0, char('}')),
    )
    .parse(input)?;
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

enum NamespaceItem<'a, NamespaceType, ClassType, VariableType, MethodType, CommentType>
where
    NamespaceType: Namespace<'a, ClassType, VariableType, MethodType, CommentType>,
    ClassType: Class<'a, VariableType, MethodType, CommentType>,
    MethodType: Method<'a, CommentType>,
    VariableType: Member<'a, CommentType>,
    CommentType: From<String>,
    Self: 'a + Sized,
{
    Ignore,
    Namespace(NamespaceType),
    Class(ClassType),
    Method(MethodType),
    Variable(VariableType),
    Comment(CommentType),
    End, // matched on `}` (+ optional `;`)

    #[doc(hidden)]
    __Phantom(std::marker::PhantomData<&'a ()>),
}

fn parse_namespace_item<'a, NamespaceType, ClassType, VariableType, MethodType, CommentType>(
    input: &'a str,
)  -> IResult<
    &'a str,
    NamespaceItem<'a, NamespaceType, ClassType, VariableType, MethodType, CommentType>,
    VerboseError<&'a str>,
>
where
    NamespaceType: Namespace<'a, ClassType, VariableType, MethodType, CommentType>,
    ClassType: Class<'a, VariableType, MethodType, CommentType>,
    MethodType: Method<'a, CommentType>,
    VariableType: Member<'a, CommentType>,
    CommentType: From<String>,
{
    let (input, item) = preceded(
        multispace0,
        alt((
            map(char(';'), |_| NamespaceItem::Ignore),
            map(parse_namespace, NamespaceItem::Namespace),
            map(parse_class, NamespaceItem::Class),
            map(parse_method, NamespaceItem::Method),
            map(parse_member, NamespaceItem::Variable),
            map(parse_comment, NamespaceItem::Comment),
            map(preceded(char('}'), opt(char(';'))), |_| NamespaceItem::End),
        )),
    )
    .parse(input)?;

    Ok((input, item))
}
