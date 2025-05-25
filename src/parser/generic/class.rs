use crate::parser::cpp::ctype::{CType, parse_cpp_type};
use crate::parser::cpp::template::parse_template;
use crate::parser::generic::annotation::Annotation;
use crate::parser::generic::comment::parse_comment;
use crate::parser::generic::member::{Member, parse_member};
use crate::parser::generic::method::{Method, parse_method};
use crate::parser::{parse_ws_str, ws};
use nom::branch::alt;
use nom::bytes::complete::{tag, take_until};
use nom::character::complete::{char, multispace0, multispace1};
use nom::combinator::{map, map_res, opt, value};
use nom::error::ParseError;
use nom::multi::{many0, separated_list1};
use nom::sequence::{delimited, preceded, terminated};
use nom::{IResult, Parser};
use nom_language::error::VerboseError;
use std::collections::HashMap;

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

pub trait Class<'a> {
    type ClassAnnotation: Annotation<'a> + 'a;
    type MemberAnnotation: Annotation<'a> + 'a;
    type Comment: From<String>;
    type Method: Method<'a> + 'a;
    type Member: Member<'a> + 'a;

    fn class(
        name: &'a str,
        api: Option<&'a str>,
        parents: Vec<CppParentClass<'a>>,
        methods: HashMap<InheritanceVisibility, Vec<Self::Method>>,
        members: HashMap<InheritanceVisibility, Vec<Self::Member>>,
        inner_classes: HashMap<InheritanceVisibility, Vec<Self>>,
        annotations: Option<Vec<Self::ClassAnnotation>>,
    ) -> Self
    where
        Self: 'a + Sized;
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct ParentClass<'a> {
    pub name: CType<'a>,
    pub visibility: InheritanceVisibility,
}

pub fn parse_class<'a, Ctx>(
    input: &'a str,
    ignore_statements: &Vec<fn(&'a str) -> IResult<&'a str, &'a str, VerboseError<&'a str>>>,
) -> IResult<&'a str, Ctx, VerboseError<&'a str>>
where
    Ctx: Class<'a> + 'a,
{
    let (input, _) = opt(parse_comment::<Ctx::Comment>).parse(input)?;
    let (input, annotations) = opt(many0(|i| Ctx::ClassAnnotation::parse(i))).parse(input)?;

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
            Class::class(
                name,
                None,
                vec![],
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
                annotations,
            ),
        ));
    }

    let (input, parents) = opt(parse_inheritance).parse(input)?;

    let mut methods: HashMap<InheritanceVisibility, Vec<Ctx::Method>> = HashMap::from([]);

    let mut members: HashMap<InheritanceVisibility, Vec<Ctx::Member>> = HashMap::from([]);
    let mut inner_classes: HashMap<InheritanceVisibility, Vec<Ctx>> = HashMap::from([]);

    // now parse the body
    let (input, _) = char('{')(input)?;
    let mut current_access = InheritanceVisibility::Private;

    let mut items = Vec::new();
    let mut input = input;

    loop {
        let trimmed = input.trim_start();

        // Early exit if block ends
        if trimmed.starts_with('}') {
            break;
        }

        match parse_class_item::<Ctx>(input, ignore_statements) {
            Ok((next_input, item)) => {
                items.push(item);
                input = next_input;
            }
            Err(e) => {
                println!("{:#?}", e);
                return Err(e)
            }, // This error now reflects the *first failing item*
        }
    }

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
        Class::class(
            name,
            api,
            parents.unwrap_or_default(),
            methods,
            members,
            inner_classes,
            annotations,
        ),
    ))
}

#[derive(Debug, PartialEq, Clone)]
enum ClassItem<'a, ClassType>
where
    ClassType: Class<'a>,
    Self: 'a + Sized,
{
    Ignore,
    Access(InheritanceVisibility),
    Method(ClassType::Method),
    Member(ClassType::Member),
    Class(ClassType),
    Comment(ClassType::Comment),
    End,
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

fn try_ignore<'a, ClassType>(
    input: &'a str,
    ignore_parsers: &Vec<fn(&'a str) -> IResult<&'a str, &'a str, VerboseError<&'a str>>>,
) -> Option<(&'a str, ClassItem<'a, ClassType>)>
where
    ClassType: Class<'a>,
{
    for parser in ignore_parsers {
        if let Ok((i, _)) = parser(input) {
            return Some((i, ClassItem::Ignore));
        }
    }
    None
}

fn parse_class_item<'a, Ctx>(
    input: &'a str,
    ignore_statements: &Vec<fn(&'a str) -> IResult<&'a str, &'a str, VerboseError<&'a str>>>,
) -> IResult<&'a str, ClassItem<'a, Ctx>, VerboseError<&'a str>>
where
    Ctx: Class<'a>,
{
    let (input, item) = preceded(
        multispace0,
        alt((
            map_res(
                |input| {
                    for parser in ignore_statements {
                        if let Ok((next_input, _)) = parser(input) {
                            return Ok((next_input, ClassItem::Ignore));
                        }
                    }
                    Err(nom::Err::Error(VerboseError::from_error_kind(
                        input,
                        nom::error::ErrorKind::Alt,
                    )))
                },
                Ok::<_, VerboseError<&str>>,
            ),
            map(alt((char(';'), char('\n'))), |_| ClassItem::Ignore),
            map(multispace1, |_| ClassItem::Ignore),
            map(access_specifier, ClassItem::Access),
            map(|i| parse_class(i, ignore_statements), ClassItem::Class),
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
