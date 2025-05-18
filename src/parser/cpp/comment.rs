#[derive(Debug, Eq, PartialEq, Clone, Default)]
pub struct CppComment {
    pub comment: String,
}

impl From<String> for CppComment {
    fn from(s: String) -> Self {
        CppComment { comment: s }
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::cpp::comment::CppComment;
    use crate::parser::generic::comment::parse_comment;

    #[test]
    fn test_parse_one_line_comment() {
        let input = "// This is a one line comment";

        let (input, comment) = parse_comment::<CppComment>(input).unwrap();
        assert!(input.is_empty());
        assert_eq!(
            comment,
            CppComment {
                comment: "This is a one line comment".to_string()
            }
        );
    }

    #[test]
    fn test_parse_one_line_comment_with_multiline_syntax() {
        let input = "/* This is a one line comment */";

        let (input, comment) = parse_comment::<CppComment>(input).unwrap();
        assert!(input.is_empty());
        assert_eq!(
            comment,
            CppComment {
                comment: "This is a one line comment ".to_string()
            }
        );
    }

    #[test]
    fn test_parse_multiline_comment_with_multiline_syntax() {
        let input = r#"/**
                             * This is a one line comment
                             */"#;

        let (input, comment) = parse_comment::<CppComment>(input).unwrap();
        assert!(input.is_empty());
        assert_eq!(
            comment,
            CppComment {
                comment: "This is a one line comment".to_string()
            }
        );
    }

    #[test]
    fn test_parse_multiple_lines_comment() {
        let input = "// This is a one line comment\n// And another line";

        let (input, comment) = parse_comment::<CppComment>(input).unwrap();
        assert!(input.is_empty());
        assert_eq!(
            comment,
            CppComment {
                comment: "This is a one line comment\nAnd another line".to_string()
            }
        );
    }
}
