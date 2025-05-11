use nom::branch::alt;
use nom::{IResult, Parser};
use nom::bytes::complete::{tag, take_until};
use nom::character::complete::{line_ending, not_line_ending};
use nom::combinator::opt;
use nom::multi::many1;
use nom::sequence::{delimited, preceded, terminated};

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct CppComment {
    pub comment: String,
}

fn strip_indent(input: &str) -> &str {
    // Remove leading whitespace, tab characters before the '*' character
    input.trim_start_matches(|c: char| c.is_whitespace() || c == '*' || c == '/')
}

/// Parses comments that starts with // or ///
fn parse_one_line_comment(input: &str) -> IResult<&str, &str> {
    let (input, line) = preceded(
        alt((tag("///"), tag("//"))),
        terminated(not_line_ending, opt(line_ending)),
    ).parse(input)?;

    Ok((input, strip_indent(line)))
}


fn parse_multiline_comment(input: &str) -> IResult<&str, Vec<&str>> {
    let (input, lines) = delimited(
        tag("/*"),
        take_until("*/"),
        tag("*/"),
    ).parse(input)?;

    let stripped_content = lines
        .lines()
        .map(|line| strip_indent(line))
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<&str>>();

    Ok((input, stripped_content))
}

pub fn parse_cpp_comment(input: &str) -> IResult<&str, CppComment> {
    let (input, lines) = alt((many1(parse_one_line_comment), parse_multiline_comment)).parse(input)?;
    Ok((input, CppComment {
        comment: lines.join("\n"),
    }))
}


#[cfg(test)]
mod tests {
    use crate::parser::cpp::comment::{parse_cpp_comment, CppComment};

    #[test]
    fn test_parse_one_line_comment() {
        let input = "// This is a one line comment";

        let (input, comment) = parse_cpp_comment(input).unwrap();
        assert!(input.is_empty());
        assert_eq!(comment, CppComment { comment: "This is a one line comment".to_string() });
    }

    #[test]
    fn test_parse_one_line_comment_with_multiline_syntax() {
        let input = "/* This is a one line comment */";

        let (input, comment) = parse_cpp_comment(input).unwrap();
        assert!(input.is_empty());
        assert_eq!(comment, CppComment { comment: "This is a one line comment ".to_string() });
    }

    #[test]
    fn test_parse_multiline_comment_with_multiline_syntax() {
        let input = r#"/**
                             * This is a one line comment
                             */"#;

        let (input, comment) = parse_cpp_comment(input).unwrap();
        assert!(input.is_empty());
        assert_eq!(comment, CppComment { comment: "This is a one line comment".to_string() });
    }

    #[test]
    fn test_parse_multiple_lines_comment() {
        let input = "// This is a one line comment\n// And another line";

        let (input, comment) = parse_cpp_comment(input).unwrap();
        assert!(input.is_empty());
        assert_eq!(comment, CppComment { comment: "This is a one line comment\nAnd another line".to_string() });
    }
}