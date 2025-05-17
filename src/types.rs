use nom::IResult;
use nom_language::error::VerboseError;
use crate::parser::cpp::ctype::CType;

/// Implement this trait for each logical code structure you want to transform into a rust struct.
/// Examples for structures are files, classes, enums, namespaces
pub trait Parsable<'a> : Sized
{
    fn parse(input: &'a str) -> IResult<&'a str, Self, VerboseError<&'a str>>;
}

/// Implement this trait for each logical struct you parsed first and for each output format.
/// Examples for output formats are plantuml, mermaid.js, graphdot.
pub trait Generatable<'a>
{
    fn generate(&self) -> &'a str;
}


/// Simple trait for all elements with a name
pub trait Named<'a> {
    fn name(&self) -> &'a str;
}

pub trait Enum<'a, Variant>: Named<'a>
where
    Variant: EnumVariant<'a>,
{
    fn get_variants(&self) -> Vec<&Variant>;
    fn get_type(variant: &Variant) -> Option<&CType<'a>>;
}

pub trait EnumVariant<'a>: Named<'a> {
    fn get_value(&self) -> Option<i64>;
}

/// Logical code block that structures related data together.
/// In most languages such as Java, C++ or Typescript this can be an interface or a class.
pub trait Struct<'a> : Named<'a> {
    /// returns all the parents of a struct.
    /// Provides an empty vector as a reference implementation for languages that don't allow
    /// inheritance
    fn get_parents(&self) -> Vec<Box<dyn Struct<'a>>> { return vec![]; }

    /// returns all methods of a struct
    fn get_methods(&self) -> Vec<Box<dyn Method<'a>>> { return vec![]; }
}

/// Code block that represents a comment
pub trait Comment<'a> : Named<'a> {
    fn get_comment(&self) -> &'a str;
}

/// Code block that represents either a function or method (function within a class).
pub trait Method<'a> {
    fn get_comment(&self) -> Option<Box<dyn Comment<'a>>>;

    fn get_generic_parameters(&self) -> Vec<CType<'a>>;

    fn get_parameters(&self) -> Vec<CType<'a>>;


    fn get_return_type(&self) -> Option<CType<'a>>;

    fn is_mutable(&self) -> bool;
}

/// Code block that represents a type (int, string, bool etc).
pub trait Type<'a> {
    fn get_type(&self) -> dyn Type<'a>;
}