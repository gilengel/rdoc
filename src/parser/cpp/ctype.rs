use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::take_while1,
    character::complete::{char, multispace0},
    combinator::opt,
    multi::many0,
    sequence::{delimited, preceded},
};
use crate::parser::cpp::is_namespace_ident_char;

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
        assert!(matches!(ty, CType::Base("String")));
    }

    #[test]
    fn test_generic() {
        let (_, ty) = ctype("String<Some>").unwrap();
        match ty {
            CType::Generic("String", inner) => {
                assert!(matches!(*inner, CType::Base("Some")));
            }
            _ => panic!("Expected generic"),
        }
    }

    #[test]
    fn test_nested_generic() {
        let (_, ty) = ctype("String<Array<Some>>").unwrap();
        match ty {
            CType::Generic("String", inner) => match *inner {
                CType::Generic("Array", inner2) => {
                    assert!(matches!(*inner2, CType::Base("Some")));
                }
                _ => panic!("Expected nested generic"),
            },
            _ => panic!("Expected generic"),
        }
    }

    #[test]
    fn test_generic_with_pointer() {
        let (_, ty) = ctype("String<Array*>").unwrap();
        match ty {
            CType::Generic("String", inner) => match *inner {
                CType::Pointer(inner2) => {
                    assert!(matches!(*inner2, CType::Base("Array")));
                }
                _ => panic!("Expected pointer in generic"),
            },
            _ => panic!("Expected generic"),
        }
    }

    #[test]
    fn test_reference() {
        let (_, ty) = ctype("String&").unwrap();
        match ty {
            CType::Reference(inner) => assert!(matches!(*inner, CType::Base("String"))),
            _ => panic!("Expected reference"),
        }
    }

    #[test]
    fn test_double_reference_nested() {
        let (_, ty) = ctype("Array<String&>&").unwrap();
        match ty {
            CType::Reference(outer) => match *outer {
                CType::Generic("Array", inner) => match *inner {
                    CType::Reference(ref_inner) => {
                        assert!(matches!(*ref_inner, CType::Base("String")));
                    }
                    _ => panic!("Expected inner reference"),
                },
                _ => panic!("Expected generic"),
            },
            _ => panic!("Expected outer reference"),
        }
    }

    #[test]
    fn test_double_reference() {
        let (_, ty) = ctype("Array&&").unwrap();
        match ty {
            CType::Reference(inner) => match *inner {
                CType::Reference(inner2) => assert!(matches!(*inner2, CType::Base("Array"))),
                _ => panic!("Expected inner reference"),
            },
            _ => panic!("Expected outer reference"),
        }
    }

    #[test]
    fn test_double_pointer() {
        let (_, ty) = ctype("Array**").unwrap();
        match ty {
            CType::Pointer(inner) => match *inner {
                CType::Pointer(inner2) => assert!(matches!(*inner2, CType::Base("Array"))),
                _ => panic!("Expected inner pointer"),
            },
            _ => panic!("Expected outer pointer"),
        }
    }
}