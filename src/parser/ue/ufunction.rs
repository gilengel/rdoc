use crate::parser::cpp::comment::CppComment;
use crate::parser::cpp::ctype::CType;
use crate::parser::generic::annotation::Annotation;

use crate::parser::cpp::method::{CppFunction, CppMethodParam};
use crate::parser::generic::method::{
    CppStorageQualifier, Method, PostParamQualifier, SpecialMember,
};
use nom::bytes::complete::{tag, take_till};
use nom::sequence::preceded;
use nom::{IResult, Parser};
use nom_language::error::VerboseError;

#[derive(Debug, PartialEq, Default, Clone)]
pub struct UFunctionAnnotation<'a>(pub Vec<&'a str>);

impl<'a> Annotation<'a> for UFunctionAnnotation<'a> {
    fn parse(input: &'a str) -> IResult<&'a str, Self, VerboseError<&'a str>> {
        // TODO fill me with life
        let (input, properties) =
            preceded(tag("UFUNCTION"), take_till(|c| c == '\n')).parse(input)?;

        Ok((input, Self(vec![properties])))
    }
}

#[derive(Debug, PartialEq)]
pub struct UFunction<'a> {
    pub function: CppFunction<'a>,
    pub annotation: UFunctionAnnotation<'a>,
}

impl<'a> Method<'a> for UFunction<'a> {
    type MethodAnnotation = UFunctionAnnotation<'a>;
    type Comment = CppComment;

    fn method(
        name: &'a str,
        return_type: Option<CType<'a>>,
        template_params: Vec<CType<'a>>,
        params: Vec<CppMethodParam<'a>>,
        storage_qualifiers: Vec<CppStorageQualifier>,
        post_param_qualifiers: Vec<PostParamQualifier>,
        special: Option<SpecialMember>,
        comment: Option<CppComment>,
        annotations: Vec<UFunctionAnnotation<'a>>,
    ) -> Self
    where
        UFunctionAnnotation<'a>: Annotation<'a> + 'a,
        CppComment: From<String>,
        Self: 'a,
    {
        let annotation = annotations.get(0).cloned().unwrap_or_default();

        UFunction {
            function: CppFunction::method(
                name,
                return_type,
                template_params,
                params,
                storage_qualifiers,
                post_param_qualifiers,
                special,
                comment,
                vec![],
            ),
            annotation,
        }
    }
}

/*
#[cfg(test)]
mod test {
    use crate::parser::cpp::ctype::CType;
    use crate::parser::cpp::member::CppMember;
    use crate::parser::generic::member::parse_member;
    use crate::parser::ue::uproperty::UMember;
    use nom::Parser;

    #[test]
    fn test_parse() {
        let input = r#"UPROPERTY(EditAnywhere, Meta = (Bitmask))
        int32 BasicBits"#;

        let result = parse_member.parse(input);
        let expected = Ok((
            "",
            UMember {
                member: CppMember {
                    name: "BasicBits",
                    ctype: CType::Path(vec!["int32"]),
                    default_value: None,
                    comment: None,
                    modifiers: vec![],
                },
                annotation: Default::default(),
            },
        ));

        assert_eq!(result, expected);
    }
}
*/
