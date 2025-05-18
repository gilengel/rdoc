use nom::IResult;
use nom_language::error::VerboseError;

/// Implement this trait for each logical code structure you want to transform into a rust struct.
/// Examples for structures are files, classes, enums, namespaces
pub trait Parsable<'a> : Sized
{
    fn parse(input: &'a str) -> IResult<&'a str, Self, VerboseError<&'a str>>;
}