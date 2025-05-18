use crate::parser::cpp::comment::CppComment;
use crate::parser::cpp::ctype::CType;
use crate::parser::cpp::member::{CppMember, CppMemberModifier};
use crate::parser::generic::annotation::Annotation;
use crate::parser::generic::member::Member;

use nom::bytes::complete::{tag, take_till};
use nom::sequence::preceded;
use nom::{IResult, Parser};
use nom_language::error::VerboseError;

#[derive(Debug, PartialEq, Default, Clone)]
pub struct UPropertyAnnotation<'a>(Vec<&'a str>);

impl<'a> Annotation<'a> for UPropertyAnnotation<'a> {
    fn parse(input: &'a str) -> IResult<&'a str, Self, VerboseError<&'a str>> {
        // TODO fill me with life
        let (input, properties) =
            preceded(tag("UPROPERTY"), take_till(|c| c == '\n')).parse(input)?;

        Ok((input, Self(vec![properties])))
    }
}

#[derive(Debug, PartialEq)]
pub struct UProperty<'a> {
    pub member: CppMember<'a>,
    pub annotation: UPropertyAnnotation<'a>,
}

impl<'a> Member<'a, UPropertyAnnotation<'a>, CppComment> for UProperty<'a> {
    fn member(
        name: &'a str,
        ctype: CType<'a>,
        default_value: Option<CType<'a>>,
        comment: Option<CppComment>,
        modifiers: Vec<CppMemberModifier>,
        annotations: Vec<UPropertyAnnotation<'a>>,
    ) -> UProperty<'a> {
        let annotation = annotations.get(0).cloned().unwrap_or_default();

        UProperty {
            member: CppMember::member(name, ctype, default_value, comment, modifiers, vec![]),
            annotation,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::parser::cpp::ctype::CType;
    use crate::parser::cpp::member::CppMember;
    use crate::parser::generic::member::parse_member;
    use crate::parser::ue::uproperty::{UProperty, UPropertyAnnotation};
    use nom::Parser;

    #[test]
    fn test_parse() {
        let input = r#"UPROPERTY(EditAnywhere, Meta = (Bitmask))
		int32 BasicBits"#;

        let result = parse_member.parse(input);
        let expected = Ok((
            "",
            UProperty {
                member: CppMember {
                    name: "BasicBits",
                    ctype: CType::Path(vec!["int32"]),
                    default_value: None,
                    comment: None,
                    modifiers: vec![],
                },
                annotation: UPropertyAnnotation(vec!["(EditAnywhere, Meta = (Bitmask))"]),
            },
        ));

        assert_eq!(result, expected);
    }
}
