use crate::parser::cpp::ctype::{CType, parse_cpp_type};
use crate::parser::cpp::template::parse_template;
use crate::parser::generic::annotation::Annotation;
use crate::parser::generic::comment::parse_comment;
use crate::parser::generic::member::{Member, parse_member};
use crate::parser::generic::method::{Method, parse_method};
use crate::parser::{parse_ws_str, ws};
use nom::branch::alt;
use nom::bytes::complete::{tag, take_until};
use nom::character::complete::{char, multispace0};
use nom::combinator::{map, opt, value};
use nom::multi::{many_till, separated_list1};
use nom::sequence::{delimited, preceded, terminated};
use nom::{IResult, Parser};
use nom_language::error::VerboseError;
use std::collections::HashMap;
use std::marker::PhantomData;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct CppParentClass<'a> {
    pub name: CType<'a>,
    pub visibility: InheritanceVisibility,
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum InheritanceVisibility {
    Private,
    Protected,
    Public,
    Virtual,
    Empty,
}

impl From<&str> for InheritanceVisibility {
    fn from(value: &str) -> Self {
        match value {
            "private" => InheritanceVisibility::Private,
            "protected" => InheritanceVisibility::Protected,
            "public" => InheritanceVisibility::Public,
            "virtual" => InheritanceVisibility::Virtual,
            _ => InheritanceVisibility::Empty,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Default)]
pub struct GClass<
    'a,
    ClassAnnotationType,
    MethodType,
    MethodAnnotationType,
    MemberType,
    MemberAnnotationType,
    CommentType,
> where
    ClassAnnotationType: Annotation<'a> + 'a,
    MethodAnnotationType: Annotation<'a> + 'a,
    MemberAnnotationType: Annotation<'a> + 'a,
    MethodType: Method<'a, MethodAnnotationType, CommentType> + 'a,
    MemberType: Member<'a, MemberAnnotationType, CommentType> + 'a,
    CommentType: From<String>,
{
    pub name: &'a str,
    pub parents: Vec<CppParentClass<'a>>,
    pub methods: HashMap<InheritanceVisibility, Vec<MethodType>>,
    pub members: HashMap<InheritanceVisibility, Vec<MemberType>>,
    pub inner_classes: HashMap<InheritanceVisibility, Vec<Self>>,
    pub annotations: Option<Vec<ClassAnnotationType>>,

    pub _marker: PhantomData<(
        CommentType,
        ClassAnnotationType,
        MethodAnnotationType,
        MemberAnnotationType,
    )>,
}

pub trait Class<
    'a,
    ClassAnnotationType,
    MethodType,
    MethodAnnotationType,
    MemberType,
    MemberAnnotationType,
    CommentType,
> where
    ClassAnnotationType: Annotation<'a> + 'a,
    MethodAnnotationType: Annotation<'a> + 'a,
    MemberAnnotationType: Annotation<'a> + 'a,
    MethodType: Method<'a, MethodAnnotationType, CommentType> + 'a,
    MemberType: Member<'a, MemberAnnotationType, CommentType> + 'a,
    CommentType: From<String>,
{
    fn class(
        name: &'a str,
        api: Option<&'a str>,
        parents: Vec<CppParentClass<'a>>,
        methods: HashMap<InheritanceVisibility, Vec<MethodType>>,
        members: HashMap<InheritanceVisibility, Vec<MemberType>>,
        inner_classes: HashMap<InheritanceVisibility, Vec<Self>>,
        annotations: Option<Vec<ClassAnnotationType>>,
    ) -> Self
    where
        Self: 'a + Sized;
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct ParentClass<'a> {
    pub name: CType<'a>,
    pub visibility: InheritanceVisibility,
}

pub fn parse_class<
    'a,
    ClassType,
    ClassAnnotationType,
    MethodType,
    MethodAnnotationType,
    MemberType,
    MemberAnnotationType,
    CommentType,
>(
    input: &'a str,
) -> IResult<&'a str, ClassType, VerboseError<&'a str>>
where
    ClassAnnotationType: Annotation<'a> + 'a,
    MethodAnnotationType: Annotation<'a> + 'a,
    MemberAnnotationType: Annotation<'a> + 'a,
    MethodType: Method<'a, MethodAnnotationType, CommentType> + 'a,
    MemberType: Member<'a, MemberAnnotationType, CommentType> + 'a,
    CommentType: From<String> + 'a,
    ClassType: Class<
            'a,
            ClassAnnotationType,
            MethodType,
            MethodAnnotationType,
            MemberType,
            MemberAnnotationType,
            CommentType,
        > + 'a,
{
    let (input, _) = opt(parse_template).parse(input)?;
    let (input, _) = parse_class_identifier(input)?;
    let (input, maybe_api) = parse_ws_str(input)?;
    let (input, maybe_name_result) = opt(parse_ws_str).parse(input)?;

    let (api, name) = match maybe_name_result {
        Some(name) => (Some(maybe_api), name), // two identifiers → api + name
        None => (None, maybe_api),             // only one → name, no api
    };

    // ignore template specialisation atm
    let input = match parse_ignore_template(input) {
        Ok((ignored, _)) => ignored,
        Err(_) => input,
    };
    let (input, _) = multispace0(input)?;

    // Return early for empty classes (e.g. forward declaration)
    let (input, empty) = opt(char::<_, VerboseError<&str>>(';')).parse(input)?;
    if empty.is_some() {
        return Ok((
            input,
            ClassType::class(
                name,
                None,
                vec![],
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
                None,
            ),
        ));
    }

    let (input, parents) = opt(parse_inheritance).parse(input)?;

    let mut methods: HashMap<InheritanceVisibility, Vec<MethodType>> = HashMap::from([]);

    let mut members: HashMap<InheritanceVisibility, Vec<MemberType>> = HashMap::from([]);
    let mut inner_classes: HashMap<InheritanceVisibility, Vec<ClassType>> = HashMap::from([]);

    // now parse the body
    let (input, _) = char('{')(input)?;
    let mut current_access = InheritanceVisibility::Private;
    let (input, (items, _)) = many_till(
        parse_class_item::<
            ClassType,
            ClassAnnotationType,
            MethodType,
            MethodAnnotationType,
            MemberType,
            MemberAnnotationType,
            CommentType,
        >,
        preceded(multispace0, char('}')),
    )
    .parse(input)?;
    for item in items {
        match item {
            ClassItem::Access(a) => current_access = a,
            ClassItem::Method(m) => methods.entry(current_access.clone()).or_default().push(m),
            ClassItem::Member(mem) => members.entry(current_access.clone()).or_default().push(mem),
            ClassItem::Class(inner_class) => inner_classes
                .entry(current_access.clone())
                .or_default()
                .push(inner_class),
            _ => {}
        }
    }

    let (input, _) = opt(char(';')).parse(input)?;

    Ok((
        input,
        ClassType::class(
            name,
            api,
            parents.unwrap_or_default(),
            methods,
            members,
            inner_classes,
            None,
        ),
    ))
}

#[derive(Debug, PartialEq, Clone)]
enum ClassItem<
    'a,
    ClassType,
    ClassAnnotationType,
    MethodType,
    MethodAnnotationType,
    MemberType,
    MemberAnnotationType,
    CommentType,
> where
    ClassAnnotationType: Annotation<'a> + 'a,
    MethodAnnotationType: Annotation<'a> + 'a,
    MemberAnnotationType: Annotation<'a> + 'a,
    MethodType: Method<'a, MethodAnnotationType, CommentType> + 'a,
    MemberType: Member<'a, MemberAnnotationType, CommentType> + 'a,
    CommentType: From<String>,
    ClassType: Class<
            'a,
            ClassAnnotationType,
            MethodType,
            MethodAnnotationType,
            MemberType,
            MemberAnnotationType,
            CommentType,
        >,
    Self: 'a + Sized,
{
    Ignore,
    Access(InheritanceVisibility),
    Method(MethodType),
    Member(MemberType),
    Class(ClassType),
    Comment(CommentType),
    End, // matched on `}` (+ optional `;`)

    #[doc(hidden)]
    __Phantom(
        std::marker::PhantomData<(
            &'a ClassAnnotationType,
            &'a MethodAnnotationType,
            &'a MemberAnnotationType,
        )>,
    ),
}

fn parse_class_identifier(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    alt((tag("class"), tag("struct"))).parse(input)
}
fn parse_inheritance_visibility(
    input: &str,
) -> IResult<&str, InheritanceVisibility, VerboseError<&str>> {
    let (input, _) = multispace0(input)?;

    // Try to match one of the known keywords
    let (input, visibility) = alt((
        value(InheritanceVisibility::Private, tag("private")),
        value(InheritanceVisibility::Protected, tag("protected")),
        value(InheritanceVisibility::Public, tag("public")),
        value(InheritanceVisibility::Virtual, tag("virtual")),
    ))
    // If no match, return Empty without consuming input
    .or(value(
        InheritanceVisibility::Empty,
        nom::combinator::success(()),
    ))
    .parse(input)?;

    Ok((input, visibility))
}

fn parse_single_inheritance(input: &str) -> IResult<&str, CppParentClass, VerboseError<&str>> {
    let (input, visibility) = parse_inheritance_visibility(input)?;
    let (input, name) = ws(parse_cpp_type).parse(input)?;

    Ok((input, CppParentClass { name, visibility }))
}
fn parse_inheritance(input: &str) -> IResult<&str, Vec<CppParentClass>, VerboseError<&str>> {
    let (input, _) = multispace0(input)?;
    let (input, _) = char(':')(input)?;
    let (input, parent_classes) =
        separated_list1(char(','), parse_single_inheritance).parse(input)?;

    Ok((input, parent_classes))
}

fn parse_class_item<
    'a,
    ClassType,
    ClassAnnotationType,
    MethodType,
    MethodAnnotationType,
    MemberType,
    MemberAnnotationType,
    CommentType,
>(
    input: &'a str,
) -> IResult<
    &'a str,
    ClassItem<
        'a,
        ClassType,
        ClassAnnotationType,
        MethodType,
        MethodAnnotationType,
        MemberType,
        MemberAnnotationType,
        CommentType,
    >,
    VerboseError<&'a str>,
>
where
    ClassAnnotationType: Annotation<'a> + 'a,
    MethodAnnotationType: Annotation<'a> + 'a,
    MemberAnnotationType: Annotation<'a> + 'a,
    MethodType: Method<'a, MethodAnnotationType, CommentType> + 'a,
    MemberType: Member<'a, MemberAnnotationType, CommentType> + 'a,
    CommentType: From<String>,
    ClassType: Class<
            'a,
            ClassAnnotationType,
            MethodType,
            MethodAnnotationType,
            MemberType,
            MemberAnnotationType,
            CommentType,
        >,
{
    let (input, item) = preceded(
        multispace0,
        alt((
            map(char(';'), |_| ClassItem::Ignore),
            map(access_specifier, ClassItem::Access),
            map(parse_class, ClassItem::Class),
            map(parse_method, ClassItem::Method),
            map(parse_member, ClassItem::Member),
            map(parse_comment, ClassItem::Comment),
            map(preceded(char('}'), opt(char(';'))), |_| ClassItem::End),
        )),
    )
    .parse(input)?;

    Ok((input, item))
}

fn access_specifier(input: &str) -> IResult<&str, InheritanceVisibility, VerboseError<&str>> {
    let (input, specifier) = map(
        terminated(
            alt((tag("public"), tag("private"), tag("protected"))),
            (tag(":"), multispace0),
        ),
        |t| InheritanceVisibility::from(t),
    )
    .parse(input)?;

    Ok((input, specifier))
}

fn parse_ignore_template(input: &str) -> IResult<&str, Option<&str>, VerboseError<&str>> {
    opt(delimited(char('<'), take_until(">"), char('>'))).parse(input)
}
