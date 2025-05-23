use crate::parser::cpp::alias::CppAlias;
use crate::parser::cpp::class::CppClass;
use crate::parser::cpp::comment::CppComment;
use crate::parser::cpp::member::CppMember;
use crate::parser::cpp::method::CppFunction;
use crate::parser::cpp::namespace::CppNamespace;
use crate::parser::generic::class::parse_class;
use crate::parser::generic::comment::parse_comment;
use crate::parser::generic::member::parse_member;
use crate::parser::generic::method::parse_method;
use crate::parser::generic::namespace::parse_namespace;
use crate::types::Parsable;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_till};
use nom::bytes::take_until;
use nom::character::complete::{char, multispace0, multispace1};
use nom::combinator::map;
use nom::sequence::{delimited, preceded, terminated};
use nom::{IResult, Parser};
use nom_language::error::VerboseError;

#[derive(Debug, Default, PartialEq)]
pub struct CppHeader<'a> {
    comments: Vec<CppComment>,
    includes: Vec<&'a str>,
    aliases: Vec<CppAlias<'a>>,
    functions: Vec<CppFunction<'a>>,
    declarations: Vec<CppMember<'a>>,
    classes: Vec<CppClass<'a>>,
    namespaces: Vec<CppNamespace<'a>>,
}

impl<'a> Parsable<'a> for CppHeader<'a> {
    fn parse(input: &'a str) -> IResult<&'a str, CppHeader<'a>, VerboseError<&'a str>> {
        let mut header = CppHeader::default();

        let mut input = input;
        loop {
            match parse_header_item(input) {
                Ok((new_rest, item)) => {
                    match item {
                        CppHeaderItem::Ignore => {}
                        CppHeaderItem::Preprocessor(_) => {}
                        CppHeaderItem::Include(inc) => header.includes.push(inc),
                        CppHeaderItem::Define(_) => {}
                        CppHeaderItem::Comment(comment) => header.comments.push(comment),
                        CppHeaderItem::Alias(alias) => header.aliases.push(alias),
                        CppHeaderItem::Function(func) => header.functions.push(func),
                        CppHeaderItem::Class(class) => header.classes.push(class),
                        CppHeaderItem::Namespace(ns) => header.namespaces.push(ns),
                        CppHeaderItem::Declaration(var) => header.declarations.push(var),
                    }
                    input = new_rest;
                }
                //Err(nom::Err::Error(_)) => return Err(), // stop on recoverable failure
                Err(e) => return Err(e), // propagate real errors
            }
        }
    }
}

#[derive(Debug)]
enum CppHeaderItem<'a> {
    Preprocessor(&'a str),
    Include(&'a str),
    Define(&'a str),
    Comment(CppComment),
    Declaration(CppMember<'a>),
    Alias(CppAlias<'a>),
    Function(CppFunction<'a>),
    Class(CppClass<'a>),
    Namespace(CppNamespace<'a>),
    Ignore,
}

fn parse_header_item(input: &str) -> IResult<&str, CppHeaderItem, VerboseError<&str>> {
    preceded(
        multispace0,
        alt((
            map(char::<_, VerboseError<&str>>('\u{feff}'), |_| {
                CppHeaderItem::Ignore
            }),
            map(parse_comment, CppHeaderItem::Comment),
            map(parse_include, CppHeaderItem::Include),
            map(parse_define, CppHeaderItem::Define),
            map(preprocessor_directive, CppHeaderItem::Preprocessor), // fallthrough for all other preprocess directives
            map(<CppAlias as Parsable>::parse, CppHeaderItem::Alias),
            map(|i| parse_class(i, &vec![]), CppHeaderItem::Class),
            map(
                terminated(parse_member, char(';')),
                CppHeaderItem::Declaration,
            ),
            map(parse_namespace, |namespace| {
                CppHeaderItem::Namespace(namespace)
            }),
            map(parse_method, CppHeaderItem::Function),
        )),
    )
    .parse(input)
}

pub fn preprocessor_directive(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    preceded(tag("#"), take_till(|c| c == '\n')).parse(input)
}

pub fn parse_define(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    preceded((tag("#define"), multispace1), take_until("\n")).parse(input)
}
pub fn parse_include(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    let (input, _) = preceded(multispace0, tag("#include")).parse(input)?;
    let (input, _) = multispace0.parse(input)?;

    let relative = delimited(char('"'), take_until("\""), char('"'));
    let absolute = delimited(char('<'), take_until(">"), char('>'));
    let (input, file) = alt((relative, absolute)).parse(input)?;

    Ok((input, file))
}

#[cfg(test)]
mod tests {
    use crate::parser::cpp::class::CppClass;
    use crate::parser::cpp::comment::CppComment;
    use crate::parser::cpp::ctype::CType;
    use crate::parser::cpp::header::{CppHeader, parse_include, preprocessor_directive};
    use crate::parser::cpp::member::CppMember;
    use crate::parser::cpp::member::CppMemberModifier::{Const, Static};
    use crate::parser::cpp::method::CppFunction;
    use crate::parser::generic::class::{CppParentClass, InheritanceVisibility};
    use crate::parser::generic::method::CppStorageQualifier::Virtual;
    use crate::parser::generic::method::PostParamQualifier::Override;
    use crate::types::Parsable;
    use std::collections::HashMap;

    #[test]
    fn test_relative_include() {
        let input = "#include \"CoreMinimal.h\"";
        let result = parse_include(input);

        assert_eq!(result, Ok(("", "CoreMinimal.h")));
    }

    #[test]
    fn test_parse_pragma() {
        let input = "#pragma once";
        let result = preprocessor_directive(input);

        assert_eq!(result, Ok(("", "pragma once")));
    }

    #[test]
    fn test_simple_header() {
        let input = r#"#pragma once
            #include "CoreMinimal.h"
            #include "Modules/ModuleManager.h"

            const static int helloCount = 0;

            struct Empty{};

            // Say hello to everyone
            void sayHello(){}

            class FCommonModule : public IModuleInterface
            {
            public:
                virtual void StartupModule() override;
                virtual void ShutdownModule() override;
            };
            "#;

        let result = CppHeader::parse(input);
        assert_eq!(
            result,
            Ok((
                "",
                CppHeader {
                    includes: vec!["CoreMinimal.h", "Modules/ModuleManager.h"],

                    classes: vec![
                        CppClass {
                            name: "Empty",
                            ..Default::default()
                        },
                        CppClass {
                            name: "FCommonModule",
                            parents: vec![CppParentClass {
                                name: CType::Path(vec!["IModuleInterface"]),
                                visibility: InheritanceVisibility::Public
                            }],
                            methods: HashMap::from([(
                                InheritanceVisibility::Public,
                                vec![
                                    CppFunction {
                                        name: "StartupModule",
                                        storage_qualifiers: vec![Virtual],
                                        post_param_qualifiers: vec![Override],
                                        ..Default::default()
                                    },
                                    CppFunction {
                                        name: "ShutdownModule",
                                        storage_qualifiers: vec![Virtual],
                                        post_param_qualifiers: vec![Override],
                                        ..Default::default()
                                    }
                                ]
                            ),]),

                            ..Default::default()
                        }
                    ],
                    functions: vec![CppFunction {
                        name: "sayHello",
                        comment: Some(CppComment {
                            comment: "Say hello to everyone".to_string()
                        }),
                        ..Default::default()
                    }],
                    declarations: vec![CppMember {
                        name: "helloCount",
                        ctype: CType::Path(vec!["int"]),
                        default_value: Some(CType::Path(vec!["0"])),
                        modifiers: vec![Const, Static],
                        ..Default::default()
                    }],
                    ..CppHeader::default()
                }
            ))
        );
    }
}
