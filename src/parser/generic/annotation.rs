use nom::IResult;
use nom_language::error::VerboseError;

pub trait Annotation<'a> where Self: Sized {
    fn parse(input: &'a str) -> IResult<&'a str, Self, VerboseError<&'a str>>;
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct NoAnnotation;
impl Annotation<'_> for NoAnnotation {
    fn parse(_: &'_ str) -> IResult<&'_ str, Self, VerboseError<&'_ str>> {
        Ok(("", NoAnnotation))
    }
}
