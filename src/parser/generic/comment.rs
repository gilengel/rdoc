use nom::branch::alt;
use nom::bytes::complete::{tag, take_until};
use nom::character::complete::{line_ending, not_line_ending};
use nom::combinator::opt;
use nom::{IResult, Parser};
use nom::multi::many1;
use nom::sequence::{delimited, preceded, terminated};
use nom_language::error::VerboseError;

pub fn parse_comment<T>(input: &str) -> IResult<&str, T, VerboseError<&str>>
where
    T: From<String>,
{
    let (input, lines) =
        alt((many1(parse_one_line_comment), parse_multiline_comment)).parse(input)?;
    Ok((input, T::from(lines.join("\n"))))
}

/// Parses comments that starts with // or ///
fn parse_one_line_comment(input: &str) -> IResult<&str, &str, VerboseError<&str>> {
    let (input, line) = preceded(
        alt((tag("///"), tag("//"))),
        terminated(not_line_ending, opt(line_ending)),
    )
        .parse(input)?;

    Ok((input, strip_indent(line)))
}

fn parse_multiline_comment(input: &str) -> IResult<&str, Vec<&str>, VerboseError<&str>> {
    let (input, lines) = delimited(tag("/*"), take_until("*/"), tag("*/")).parse(input)?;

    let stripped_content = lines
        .lines()
        .map(|line| strip_indent(line))
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<&str>>();

    Ok((input, stripped_content))
}



fn strip_indent(input: &str) -> &str {
    // Remove leading whitespace, tab characters before the '*' character
    input.trim_start_matches(|c: char| c.is_whitespace() || c == '*' || c == '/')
}