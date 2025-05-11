use nom::combinator::map;
use nom::multi::separated_list0;
use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{char, multispace0},
    combinator::opt,
    multi::many0,
    sequence::{delimited, preceded},
};

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum CType<'a> {
    Path(Vec<&'a str>),                       // std::enable_if_t
    Generic(Box<CType<'a>>, Vec<CType<'a>>),  // std::vector<int>
    Function(Box<CType<'a>>, Vec<CType<'a>>), // return‐type + params
    Pointer(Box<CType<'a>>),                  // *
    Reference(Box<CType<'a>>),                // &
    MemberAccess(Box<CType<'a>>, &'a str),    // ...::value
}

impl Default for CType<'static> {
    fn default() -> Self {
        CType::Path(Vec::new())
    }
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn identifier(input: &str) -> IResult<&str, &str> {
    take_while1(is_ident_char)(input)
}

fn path(input: &str) -> IResult<&str, CType> {
    let (input, ctype) = map(separated_list0(tag("::"), identifier), CType::Path).parse(input)?;

    Ok((input, ctype))
}

fn parse_generics<'a>(input: &'a str, ty: CType<'a>) -> IResult<&'a str, CType<'a>> {
    opt(delimited(
        preceded(multispace0, char('<')),
        separated_list0(
            preceded(multispace0, char(',')),
            preceded(multispace0, parse_cpp_type),
        ),
        preceded(multispace0, char('>')),
    ))
    .parse(input)
    .map(|(rest, args_opt)| {
        let out = match args_opt {
            Some(args) => CType::Generic(Box::new(ty), args),
            None => ty,
        };
        (rest, out)
    })
}

fn parse_member_access<'a>(input: &'a str, mut ty: CType<'a>) -> IResult<&'a str, CType<'a>> {
    let (input, members) = many0(preceded(tag("::"), identifier)).parse(input)?;
    for m in members {
        ty = CType::MemberAccess(Box::new(ty), m);
    }
    Ok((input, ty))
}

fn parse_function<'a>(input: &'a str, ty: CType<'a>) -> IResult<&'a str, CType<'a>> {
    opt(delimited(
        preceded(multispace0, char('(')),
        separated_list0(
            preceded(multispace0, char(',')),
            preceded(multispace0, parse_cpp_type),
        ),
        preceded(multispace0, char(')')),
    ))
    .parse(input)
    .map(|(rest, params_opt)| {
        let out = match params_opt {
            Some(params) => CType::Function(Box::new(ty), params),
            None => ty,
        };
        (rest, out)
    })
}

fn parse_ptrs_refs<'a>(mut ty: CType<'a>, input: &'a str) -> IResult<&'a str, CType<'a>> {
    let (input, suffixes) =
        many0(preceded(multispace0, alt((char('*'), char('&'))))).parse(input)?;
    for s in suffixes {
        ty = match s {
            '*' => CType::Pointer(Box::new(ty)),
            '&' => CType::Reference(Box::new(ty)),
            _ => unreachable!(),
        };
    }
    Ok((input, ty))
}

fn parse_type_atom(input: &str) -> IResult<&str, CType> {
    // First, parse path
    let (input, ctype) = path(input)?;

    // Recurse into optional generics
    let (input, ctype) = parse_generics(input, ctype)?;

    // Recurse into optional member accesses
    let (input, ctype) = parse_member_access(input, ctype)?;

    // Handle function type like T(A, B)
    let (input, ctype) = parse_function(input, ctype)?;

    // Parse any trailing pointers/references
    let (input, ctype) = parse_ptrs_refs(ctype, input)?;

    Ok((input, ctype))
}

pub fn parse_cpp_type(input: &str) -> IResult<&str, CType> {
    parse_type_atom(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::cpp::ctype::CType::{Function, Generic, MemberAccess, Path, Reference};

    #[test]
    fn test_simple_identifier() {
        let (_, ty) = parse_cpp_type("String").unwrap();
        assert_eq!(ty, Path(vec!["String"]));
    }

    #[test]
    fn test_generic() {
        let (_, ty) = parse_cpp_type("String<Some>").unwrap();
        assert_eq!(
            ty,
            Generic(Box::new(Path(vec!["String"])), vec![Path(vec!["Some"])])
        );
    }

    #[test]
    fn test_nested_generic() {
        let (_, ty) = parse_cpp_type("String<Array<Some>>").unwrap();
        assert_eq!(
            ty,
            Generic(
                Box::new(Path(vec!["String"])),
                vec![Generic(
                    Box::from(Path(vec!["Array"])),
                    vec![Path(vec!["Some"])]
                )]
            )
        );
    }

    #[test]
    fn test_generic_with_pointer() {
        let (_, ty) = parse_cpp_type("String<Array*>").unwrap();
        assert_eq!(
            ty,
            Generic(
                Box::new(Path(vec!["String"])),
                vec![CType::Pointer(Box::new(Path(vec!["Array"])))]
            )
        );
    }

    #[test]
    fn test_reference() {
        let (_, ty) = parse_cpp_type("String&").unwrap();
        assert_eq!(ty, Reference(Box::new(Path(vec!["String"]))))
    }

    #[test]
    fn test_double_reference_nested() {
        let (_, ty) = parse_cpp_type("Array<String&>&").unwrap();
        assert_eq!(
            ty,
            Reference(Box::new(Generic(
                Box::new(Path(vec!["Array"])),
                vec![Reference(Box::new(Path(vec!["String"])))]
            )))
        )
    }

    #[test]
    fn test_double_reference() {
        let (_, ty) = parse_cpp_type("Array&&").unwrap();
        assert_eq!(
            ty,
            Reference(Box::new(Reference(Box::new(Path(vec!["Array"])))))
        );
    }

    #[test]
    fn test_double_pointer() {
        let (_, ty) = parse_cpp_type("Array**").unwrap();
        assert_eq!(
            ty,
            CType::Pointer(Box::new(CType::Pointer(Box::new(Path(vec!["Array"])))))
        );
    }

    #[test]
    fn test_lambda() {
        let (input, ty) = parse_cpp_type("std::function<int(int)>&").unwrap();

        assert_eq!(input, "");
        assert_eq!(
            ty,
            Reference(Box::from(Generic(
                Box::from(Path(vec!["std", "function"])),
                vec![Function(
                    Box::from(Path(vec!["int"])),
                    vec![Path(vec!["int"])]
                )]
            )))
        );
    }

    #[test]
    fn test_complex_cpp_type() {
        let (input, ty) =
            parse_cpp_type("std::enable_if_t<std::is_integral<Integer>::value>").unwrap();
        assert_eq!(input, "");
        assert_eq!(
            ty,
            Generic(
                Box::from(Path(vec!["std", "enable_if_t"])),
                vec![MemberAccess(
                    Box::from(Generic(
                        Box::from(Path(vec!["std", "is_integral"])),
                        vec![Path(vec!["Integer"])]
                    )),
                    "value"
                )]
            )
        );
    }
}
