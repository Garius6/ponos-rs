pub mod combinator;
pub mod error;
pub mod expression;
pub mod lexer;
pub mod statement;
pub mod types;

use crate::ponos::ast::Program;
use combinator::{Input, PResult, ws};
pub use error::{ParseErrorKind, PonosParseError};
use winnow::Parser;

/// Главный парсер Ponos
pub struct PonosParser {
    // Состояние парсера (если потребуется)
}

impl PonosParser {
    pub fn new() -> Self {
        PonosParser {}
    }

    /// Парсит исходный код в AST
    pub fn parse(&mut self, source: String) -> Result<Program, PonosParseError> {
        let mut input = source.as_str();

        match parse_program(&mut input) {
            Ok(program) => {
                // Проверяем, что весь ввод обработан
                ws(&mut input).ok();
                if !input.is_empty() {
                    return Err(PonosParseError::new(
                        ParseErrorKind::UnexpectedToken {
                            expected: vec!["<конец файла>".to_string()],
                            found: input.chars().take(10).collect(),
                        },
                        crate::ponos::span::Span::new(source.len() - input.len(), source.len()),
                    ));
                }
                Ok(program)
            }
            Err(e) => {
                if let winnow::error::ErrMode::Backtrack(err) = e {
                    Err(err)
                } else {
                    Err(PonosParseError::new(
                        ParseErrorKind::Custom("Неизвестная ошибка парсинга".to_string()),
                        crate::ponos::span::Span::default(),
                    ))
                }
            }
        }
    }
}

impl Default for PonosParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Парсит программу (список операторов)
fn parse_program<'a>(input: &mut Input<'a>) -> PResult<'a, Program> {
    use crate::ponos::parser::lexer::skip_ws_and_comments;
    use winnow::combinator::repeat;

    skip_ws_and_comments(input)?;

    // Парсим список операторов
    let statements: Vec<_> = repeat(0.., |input: &mut Input<'a>| {
        skip_ws_and_comments(input)?;
        let stmt = statement::parse_statement(input)?;
        skip_ws_and_comments(input)?;
        Ok(stmt)
    })
    .parse_next(input)?;

    Ok(Program { statements })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_program() {
        let mut parser = PonosParser::new();
        let result = parser.parse("".to_string());
        assert!(result.is_ok());

        let program = result.unwrap();
        assert_eq!(program.statements.len(), 0);
    }

    #[test]
    fn test_whitespace_only() {
        let mut parser = PonosParser::new();
        let result = parser.parse("   \n\t  ".to_string());
        assert!(result.is_ok());
    }
}
