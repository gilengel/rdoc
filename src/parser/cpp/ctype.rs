use crate::parser::cpp::is_namespace_ident_char;
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::take_while1,
    character::complete::{char, multispace0},
    combinator::opt,
    multi::many0,
    sequence::{delimited, preceded},
};

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum CType<'a> {
    Base(&'a str),
    Generic(&'a str, Box<CType<'a>>),
    Pointer(Box<CType<'a>>),
    Reference(Box<CType<'a>>),
}

fn identifier(input: &str) -> IResult<&str, &str> {
    take_while1(is_namespace_ident_char)(input)
}

pub fn ctype(input: &str) -> IResult<&str, CType> {
    let (input, base) = identifier(input)?;

    // Parse optional generic part: <Type>
    let (input, generics) = opt(delimited(
        preceded(multispace0, char('<')),
        preceded(multispace0, ctype),
        preceded(multispace0, char('>')),
    ))
    .parse(input)?;

    let mut typ = match generics {
        Some(inner) => CType::Generic(base, Box::new(inner)),
        None => CType::Base(base),
    };

    // Parse 0 or more *, &, etc.
    let (input, suffixes) =
        many0(preceded(multispace0, alt((char('*'), char('&'))))).parse(input)?;

    for suffix in suffixes {
        typ = match suffix {
            '*' => CType::Pointer(Box::new(typ)),
            '&' => CType::Reference(Box::new(typ)),
            _ => unreachable!(),
        };
    }

    Ok((input, typ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_identifier() {
        let (_, ty) = ctype("String").unwrap();
        assert_eq!(ty, CType::Base("String"));
    }

    #[test]
    fn test_generic() {
        let (_, ty) = ctype("String<Some>").unwrap();
        assert_eq!(ty, CType::Generic("String", Box::new(CType::Base("Some"))));
    }

    #[test]
    fn test_nested_generic() {
        let (_, ty) = ctype("String<Array<Some>>").unwrap();
        assert_eq!(ty, CType::Generic("String", Box::new(CType::Generic("Array", Box::new(CType::Base("Some"))))));
    }

    #[test]
    fn test_generic_with_pointer() {
        let (_, ty) = ctype("String<Array*>").unwrap();
        assert_eq!(ty, CType::Generic("String", Box::new(CType::Pointer(Box::new(CType::Base("Array"))))));
    }

    #[test]
    fn test_reference() {
        let (_, ty) = ctype("String&").unwrap();
        assert_eq!(ty, CType::Reference(Box::new(CType::Base("String"))))
    }

    #[test]
    fn test_double_reference_nested() {
        let (_, ty) = ctype("Array<String&>&").unwrap();
        assert_eq!(
            ty,
            CType::Reference(Box::new(CType::Generic(
                "Array",
                Box::new(CType::Reference(Box::new(CType::Base("String"))))
            )))
        )
    }

    #[test]
    fn test_double_reference() {
        let (_, ty) = ctype("Array&&").unwrap();
        assert_eq!(
            ty,
            CType::Reference(Box::new(CType::Reference(Box::new(CType::Base("Array")))))
        );
    }

    #[test]
    fn test_double_pointer() {
        let (_, ty) = ctype("Array**").unwrap();
        assert_eq!(
            ty,
            CType::Pointer(Box::new(CType::Pointer(Box::new(CType::Base("Array")))))
        );
    }

    #[test]
    fn test_lambda() {
        let (_, ty) = ctype("std::function<int(int>&").unwrap();
        assert_eq!(ty, CType::Base("std::function"));
    }
}
