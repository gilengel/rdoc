﻿use crate::parser::cpp::comment::CppComment;
use crate::parser::generic::annotation::Annotation;
use crate::parser::generic::class::{Class, CppParentClass, InheritanceVisibility};
use crate::parser::ue::ufunction::{UFunction};
use crate::parser::ue::uproperty::{UProperty, UPropertyAnnotation};
use nom::bytes::complete::{tag, take_till};
use nom::character::complete::multispace0;
use nom::sequence::preceded;
use nom::{IResult, Parser};
use nom_language::error::VerboseError;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Default, Clone)]
pub struct UClassAnnotation<'a>(Vec<&'a str>);

impl<'a> Annotation<'a> for UClassAnnotation<'a> {
    fn parse(input: &'a str) -> IResult<&'a str, Self, VerboseError<&'a str>> {
        // TODO fill me with life
        let (input, properties) = preceded(tag("UCLASS"), take_till(|c| c == '\n')).parse(input)?;
        let (input, _) = multispace0(input)?;

        Ok((input, Self(vec![properties])))
    }
}

#[derive(Debug, PartialEq)]
pub struct UClass<'a> {
    pub name: &'a str,
    pub api: Option<&'a str>,
    pub parents: Vec<CppParentClass<'a>>,
    pub methods: HashMap<InheritanceVisibility, Vec<UFunction<'a>>>,
    pub members: HashMap<InheritanceVisibility, Vec<UProperty<'a>>>,
    pub inner_classes: HashMap<InheritanceVisibility, Vec<UClass<'a>>>,
    pub annotation: UClassAnnotation<'a>,
}

impl Default for UClass<'_> {
    fn default() -> Self {
        Self {
            name: "",
            api: None,
            parents: vec![],
            methods: HashMap::from([]),
            members: HashMap::from([]),
            inner_classes: HashMap::from([]),
            annotation: Default::default(),
        }
    }
}

impl<'a> Class<'a> for UClass<'a> {
    type ClassAnnotation = UClassAnnotation<'a>;
    type MemberAnnotation = UPropertyAnnotation<'a>;
    type Comment = CppComment;
    type Method = UFunction<'a>;
    type Member = UProperty<'a>;

    fn class(
        name: &'a str,
        api: Option<&'a str>,
        parents: Vec<CppParentClass<'a>>,
        methods: HashMap<InheritanceVisibility, Vec<Self::Method>>,
        members: HashMap<InheritanceVisibility, Vec<Self::Member>>,
        inner_classes: HashMap<InheritanceVisibility, Vec<Self>>,
        annotation: Option<Vec<Self::ClassAnnotation>>,
    ) -> Self
    where
        Self: 'a + Sized,
    {
        let default = UClassAnnotation(vec![]); // this now lives long enough
        let annotation = annotation
            .unwrap_or_default()
            .get(0)
            .unwrap_or(&default) // now `&default` has a valid lifetime
            .clone();

        Self {
            name,
            api,
            parents,
            methods,
            members,
            inner_classes,
            annotation,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::cpp::comment::CppComment;
    use crate::parser::cpp::ctype::CType;
    use crate::parser::cpp::method::{CppFunction, CppMethodParam};
    use crate::parser::generic::class::InheritanceVisibility::{Protected, Public};
    use crate::parser::generic::class::{CppParentClass, parse_class};
    use crate::parser::generic::method::CppStorageQualifier::Virtual;
    use crate::parser::generic::method::PostParamQualifier::Override;
    use crate::parser::ue::uclass::{UClass, UClassAnnotation};
    use crate::parser::ue::ufunction::{UFunction, UFunctionAnnotation};
    use nom::bytes::complete::tag;
    use nom::character::complete::multispace0;
    use nom::sequence::preceded;
    use nom::{IResult, Parser};
    use nom_language::error::VerboseError;
    use std::collections::HashMap;

    #[test]
    fn test_parse_empty_cpp_with_inheritance_class() {
        let input = r#"UCLASS()
class COMMON_API AClass : public AActor
{
    GENERATED_BODY()
public:
	// Sets default values for this character's properties
	AClass(const FObjectInitializer& ObjectInitializer);
protected:
	// Called when the game starts or when spawned
	virtual void BeginPlay() override;
};"#;

        let expected = UClass {
            name: "AClass",
            api: Some("COMMON_API"),
            parents: vec![CppParentClass {
                name: CType::Path(vec!["AActor"]),
                visibility: Public,
            }],
            methods: HashMap::from([
                (
                    Public,
                    vec![UFunction {
                        function: CppFunction {
                            name: "AClass",
                            params: vec![CppMethodParam {
                                name: Some("ObjectInitializer"),
                                ctype: CType::Const(Box::from(CType::Reference(Box::from(
                                    CType::Path(vec!["FObjectInitializer"]),
                                )))),
                                default_value: None,
                            }],
                            comment: Some(CppComment {
                                comment: "Sets default values for this character's properties"
                                    .to_string(),
                            }),
                            ..Default::default()
                        },
                        annotation: UFunctionAnnotation(vec![]),
                    }],
                ),
                (
                    Protected,
                    vec![UFunction {
                        function: CppFunction {
                            name: "BeginPlay",
                            storage_qualifiers: vec![Virtual],
                            post_param_qualifiers: vec![Override],
                            comment: Some(CppComment {
                                comment: "Called when the game starts or when spawned".to_string(),
                            }),
                            ..Default::default()
                        },
                        annotation: UFunctionAnnotation(vec![]),
                    }],
                ),
            ]),
            members: Default::default(),
            inner_classes: Default::default(),
            annotation: UClassAnnotation(vec!["()"]),
        };

        let ignore_generate_body = |input| -> IResult<&str, &str, VerboseError<&str>> {
            preceded(tag("GENERATED_BODY()"), multispace0).parse(input)
        };
        assert_eq!(
            parse_class(input, &vec![ignore_generate_body]),
            Ok(("", expected))
        );
    }
}
/*
        for visibility in ["private", "protected", "public", ""] {
            let input = format!("class test : {visibility} a {{}};");
            let result = parse_class(&input);

            assert_eq!(
                result,
                Ok((
                    "",
                    CppClass {
                        name: "test",
                        api: None,
                        parents: vec![CppParentClass {
                            name: Path(vec!["a"]),
                            visibility: InheritanceVisibility::from(visibility)
                        }],
                        ..CppClass::default()
                    }
                ))
            );
        }
    }
*/
