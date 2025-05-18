use crate::parser::cpp::alias::CppAlias;
use crate::parser::cpp::class::CppClass;
use crate::parser::cpp::member::CppMember;
use crate::parser::cpp::method::CppFunction;
use crate::parser::cpp::namespace::CppNamespace;
use crate::parser::generic::class::parse_class;
use crate::parser::generic::member::parse_member;
use crate::parser::generic::method::parse_method;
use crate::parser::generic::namespace::parse_namespace;
use crate::types::Parsable;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_till};
use nom::bytes::take_until;
use nom::character::complete::{char, multispace0, multispace1};
use nom::combinator::map;
use nom::multi::fold_many0;
use nom::sequence::{delimited, preceded, terminated};
use nom::{IResult, Parser};
use nom_language::error::VerboseError;

#[derive(Debug, Default, PartialEq)]
pub struct CppHeader<'a> {
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

        let (input, _) = fold_many0(
            parse_header_item,
            move || (),
            |_, item| match item {
                CppHeaderItem::Ignore => {}
                CppHeaderItem::Pragma(_) => {}
                CppHeaderItem::Include(include) => {
                    header.includes.push(include);
                }
                CppHeaderItem::Define(_) => {}
                CppHeaderItem::Alias(alias) => {
                    header.aliases.push(alias);
                }
                CppHeaderItem::Function(function) => {
                    header.functions.push(function);
                }
                CppHeaderItem::Class(class) => {
                    header.classes.push(class);
                }
                CppHeaderItem::Namespace(namespace) => {
                    header.namespaces.push(namespace);
                }
                CppHeaderItem::Declaration(variable) => {
                    header.declarations.push(variable);
                }
            },
        )
        .parse(input)?;

        let (input, _) = multispace0(input)?;

        Ok((input, header))
    }
}

#[derive(Debug)]
enum CppHeaderItem<'a> {
    Pragma(&'a str),
    Include(&'a str),
    Define(&'a str),
    Declaration(CppMember<'a>),
    Alias(CppAlias<'a>),
    Function(CppFunction<'a>),
    Class(CppClass<'a>),
    Namespace(CppNamespace<'a>),
    Ignore,
}

fn parse_header_item(input: &str) -> IResult<&str, CppHeaderItem, VerboseError<&str>> {
    let (input, item) = preceded(
        multispace0,
        alt((
            map(parse_pragma, |pragma| CppHeaderItem::Pragma(pragma)),
            map(parse_include, |include| CppHeaderItem::Include(include)),
            map(parse_define, |define| CppHeaderItem::Define(define)),
            map(<CppAlias as Parsable>::parse, CppHeaderItem::Alias),
            map(|i| parse_class(i, &vec![]), CppHeaderItem::Class),
            map(
                terminated(parse_member, char(';')),
                CppHeaderItem::Declaration,
            ),
            map(parse_namespace, CppHeaderItem::Namespace),
            map(parse_method, CppHeaderItem::Function),
            map(tag("\n"), |_| CppHeaderItem::Ignore),
        )),
    )
    .parse(input)?;

    Ok((input, item))
}

pub fn parse_pragma(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    preceded(tag("#pragma"), take_till(|c| c == '\n')).parse(input)
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
    use crate::parser::cpp::header::{CppHeader, parse_include, parse_pragma};
    use crate::parser::cpp::member::CppMember;
    use crate::parser::cpp::member::CppMemberModifier::{Const, Static};
    use crate::parser::cpp::method::{CppFunction, CppFunctionInheritance};
    use crate::parser::generic::class::{CppParentClass, InheritanceVisibility};
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
        let result = parse_pragma(input);

        assert_eq!(result, Ok(("", " once")));
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
                                        inheritance_modifiers: vec![
                                            CppFunctionInheritance::Virtual,
                                            CppFunctionInheritance::Override
                                        ],
                                        ..Default::default()
                                    },
                                    CppFunction {
                                        name: "ShutdownModule",
                                        inheritance_modifiers: vec![
                                            CppFunctionInheritance::Virtual,
                                            CppFunctionInheritance::Override
                                        ],
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
