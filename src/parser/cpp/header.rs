use nom::bytes::complete::tag;
use nom::{IResult, Parser};
use nom::branch::alt;
use nom::bytes::take_until;
use nom::character::complete::{char, multispace0};
use nom::combinator::eof;
use nom::sequence::{delimited, preceded, terminated};
use crate::parser::cpp::class::{parse_cpp_class, CppClass};
use crate::parser::cpp::comment::parse_cpp_comment;
use crate::parser::cpp::method::{parse_cpp_method, CppFunction};
use crate::parser::cpp::ws;

#[derive(Debug)]
pub struct CppHeader<'a> {
    functions: Vec<CppFunction<'a>>,
    classes: Vec<CppClass<'a>>,
}
pub fn parse_pragma_once(input: &str) -> IResult<&str, &str> {
    tag("#pragma once")(input)
}

pub fn parse_include(input: &str) -> IResult<&str, &str> {
    let (input, _) = preceded(multispace0, tag("#include")).parse(input)?;
    let (input, _) = multispace0.parse(input)?;

    let relative = delimited(char('"'), take_until("\""), char('"'));
    let absolute = delimited(char('<'), take_until(">"), char('>'));
    let (input, file) = alt((relative, absolute)).parse(input)?;

    Ok((input, file))
}

pub fn parse_uclass(input: &str) -> IResult<&str, &str>
{
    let (input, _) = (ws(tag("UCLASS")), take_until("\n")).parse(input)?;

    Ok((input, ""))
}

pub fn parse_cpp_header(input: &str) -> IResult<&str, CppHeader> {
    let mut input = input;

    let mut classes = Vec::new();
    let mut functions = Vec::new();
    loop {
        let (i, _) = multispace0(input)?;
        input = i;

        if let Ok((i, _)) = (multispace0, parse_pragma_once, multispace0).parse(input) {
            input = i;
            continue;
        }

        if let Ok((i, _)) = parse_uclass(input) {
            input = i;
            continue
        }

        if let Ok((i, _)) =  preceded(multispace0, parse_include).parse(input) {
            input = i;
            continue
        }

        if let Ok((i, _)) = preceded(multispace0, parse_cpp_comment).parse(input) {
            input = i;
            continue
        }

        if let Ok((i, class)) = preceded(multispace0, parse_cpp_class).parse(input) {
            classes.push(class);

            input = i;
            continue
        }

        if let Ok((i, function)) = terminated(parse_cpp_method, char(';')).parse(input) {
            functions.push(function);

            input = i;
            continue
        }

        if let Ok((i, _)) = eof::<&str, nom::error::Error<&str>>(input) {
            return Ok((i, CppHeader { functions, classes })); // Successfully reached EOF
        }

        return Err(nom::Err::Error(nom::error::Error::new(
            i,
            nom::error::ErrorKind::Tag,
        )))
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::cpp::header::{parse_cpp_header, parse_include};
    use crate::parser::cpp::method::CppFunction;

    #[test]
    fn test_relative_include() {
        let input = "#include \"CoreMinimal.h\"";
        let result = parse_include(input);

        assert_eq!(result, Ok(("", "CoreMinimal.h")));
    }


    #[test]
    fn test_simple_header() {
        let input = r#"#pragma once
            #include "CoreMinimal.h"
            #include "Modules/ModuleManager.h"

            struct Empty{};

            // Say hello to everyone
            void sayHello();

            class FCommonModule : public IModuleInterface
            {
            public:
                virtual void StartupModule() override;
                virtual void ShutdownModule() override;
            };
            "#;

        let result = parse_cpp_header(input).unwrap();
        assert_eq!(result.0, "");
        assert_eq!(result.1.classes.len(), 2);
        assert_eq!(result.1.functions, vec![CppFunction {
            name: "sayHello",
            ..Default::default()
        }])
    }
}