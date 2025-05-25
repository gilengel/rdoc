use crate::parser::cpp::comment::CppComment;
use crate::parser::cpp::ctype::CType;
use crate::parser::generic::annotation::NoAnnotation;
use crate::parser::generic::member::Member;

#[derive(Debug, Eq, PartialEq, Clone, Default)]
pub struct CppMember<'a> {
    pub name: &'a str,
    pub ctype: CType<'a>,
    pub default_value: Option<CType<'a>>,
    pub comment: Option<CppComment>,
    pub modifiers: Vec<CppMemberModifier>,
}

impl<'a> Member<'a> for CppMember<'a> {
    type Annotation = NoAnnotation;
    type Comment = CppComment;

    fn member(
        name: &'a str,
        ctype: CType<'a>,
        default_value: Option<CType<'a>>,
        comment: Option<CppComment>,
        modifiers: Vec<CppMemberModifier>,
        _annotations: Vec<NoAnnotation>,
    ) -> Self
    where
        Self: 'a,
    {
        Self {
            name,
            ctype,
            default_value,
            comment,
            modifiers,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]

pub enum CppMemberModifier {
    Static,
    Const,
    Inline,
}

impl Into<String> for CppMemberModifier {
    fn into(self) -> String {
        match self {
            CppMemberModifier::Static => "static".to_string(),
            CppMemberModifier::Const => "const".to_string(),
            CppMemberModifier::Inline => "inline".to_string(),
        }
    }
}
impl From<&str> for CppMemberModifier {
    fn from(value: &str) -> Self {
        match value {
            "static" => CppMemberModifier::Static,
            "const" => CppMemberModifier::Const,
            "inline" => CppMemberModifier::Inline,
            _ => unimplemented!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::cpp::ctype::CType::Path;
    use crate::parser::cpp::member::{CppMember, CppMemberModifier};
    use crate::parser::generic::member::parse_member;

    #[test]
    fn test_cpp_member_without_default_value() {
        let input = "int member";
        assert_eq!(
            parse_member(input),
            Ok((
                "",
                CppMember {
                    name: "member",
                    ctype: Path(vec!["int"]),
                    ..Default::default()
                }
            ))
        );
    }

    #[test]
    fn test_cpp_member_with_modifier() {
        for modifier in vec!["static", "const", "inline"] {
            let input = format!("{} int member", modifier);
            assert_eq!(
                parse_member(&input),
                Ok((
                    "",
                    CppMember {
                        name: "member",
                        ctype: Path(vec!["int"]),
                        modifiers: vec![CppMemberModifier::from(modifier)],
                        ..Default::default()
                    }
                ))
            );
        }
    }

    #[test]
    fn test_cpp_member_with_default_value() {
        for input in ["int member = 0", "int member {0}"] {
            assert_eq!(
                parse_member(input),
                Ok((
                    "",
                    CppMember {
                        name: "member",
                        ctype: Path(vec!["int"]),
                        default_value: Some(Path(vec!["0"])),
                        ..Default::default()
                    }
                ))
            );
        }
    }
}
