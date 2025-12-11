pub mod combinator;
pub mod error;
pub mod expression;
pub mod lexer;
pub mod statement;
pub mod types;

use winnow::Parser;
use crate::ponos::ast::Program;
pub use error::{PonosParseError, ParseErrorKind};
use combinator::{ws, Input, PResult};

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
        let preprocessed = preprocess_semicolons(&source);
        let mut input = preprocessed.as_str();

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
                        crate::ponos::span::Span::new(
                            source.len() - input.len(),
                            source.len(),
                        ),
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
    use winnow::combinator::repeat;
    use crate::ponos::parser::lexer::skip_ws_and_comments;

    skip_ws_and_comments(input)?;

    // Парсим список операторов
    let statements: Vec<_> = repeat(0.., |input: &mut Input<'a>| {
        skip_ws_and_comments(input)?;
        let stmt = statement::parse_statement(input)?;
        skip_ws_and_comments(input)?;
        Ok(stmt)
    }).parse_next(input)?;

    Ok(Program {
        statements,
    })
}

/// Вставляет виртуальные точки с запятой по правилам, похожим на Go:
/// после завершённого токена перед переводом строки/EOF, если строка не продолжается оператором.
fn preprocess_semicolons(source: &str) -> String {
    let mut out = String::with_capacity(source.len() + 8);
    let mut chars = source.chars().peekable();

    #[derive(Clone, Copy)]
    enum LastTokenEnd {
        Identifier,
        Number,
        String,
        CloseDelim, // ) ] }
        Other(char),
    }

    let mut last_end: Option<LastTokenEnd> = None;
    let mut block_header = false; // после ключевых слов, открывающих блок, не вставляем ';' на первой новой строке
    let mut last_ident: Option<String> = None;
    let block_keywords = [
        "если", "иначе", "пока", "попытка", "перехват", "функ", "конструктор", "класс",
        "интерфейс", "аннотация",
    ];
    let no_semicolon_after = [
        "конец", "иначе", "перехват", "попытка", "если", "пока", "функ", "конструктор", "класс",
        "интерфейс", "аннотация",
    ];

    let should_insert =
        |last: Option<LastTokenEnd>, block_header: bool, last_ident: &Option<String>| -> bool {
            if block_header {
                return false;
            }
            if let Some(ident) = last_ident {
                if no_semicolon_after.contains(&ident.as_str()) {
                    return false;
                }
            }
            match last {
                Some(LastTokenEnd::Identifier)
                | Some(LastTokenEnd::Number)
                | Some(LastTokenEnd::String)
                | Some(LastTokenEnd::CloseDelim) => true,
                Some(LastTokenEnd::Other(c)) => !matches!(
                    c,
                    ';'
                        | '('
                        | '['
                        | '{'
                        | ','
                        | '.'
                        | '+'
                        | '-'
                        | '*'
                        | '/'
                        | '%'
                        | '<'
                        | '>'
                        | '='
                        | '!'
                        | '&'
                        | '|'
                        | ':'
                ),
                None => false,
            }
        };

    while let Some(c) = chars.next() {
        // Строковые литералы
        if c == '"' {
            out.push(c);
            last_end = Some(LastTokenEnd::String);
            let mut escaped = false;
            while let Some(ch) = chars.next() {
                out.push(ch);
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    break;
                }
            }
            continue;
        }

        // Комментарии
        if c == '/' {
            if let Some('/') = chars.peek().copied() {
                // однострочный комментарий
                out.push('/');
                out.push('/');
                chars.next();
                while let Some(ch) = chars.next() {
                    if ch == '\n' {
                        if should_insert(last_end, block_header, &last_ident) {
                            out.push(';');
                            last_end = Some(LastTokenEnd::Other(';'));
                            last_ident = None;
                        }
                        out.push('\n');
                        block_header = false;
                        last_ident = None;
                        last_end = None;
                        break;
                    }
                    out.push(ch);
                }
                continue;
            } else if let Some('*') = chars.peek().copied() {
                // многострочный комментарий
                out.push('/');
                out.push('*');
                chars.next();
                while let Some(ch) = chars.next() {
                    out.push(ch);
                    if ch == '*' {
                        if let Some('/') = chars.peek().copied() {
                            out.push('/');
                            chars.next();
                            break;
                        }
                    }
                }
                continue;
            }
        }

        if c == '\n' {
            if should_insert(last_end, block_header, &last_ident) {
                out.push(';');
                last_end = Some(LastTokenEnd::Other(';'));
                last_ident = None;
            }
            out.push('\n');
            block_header = false;
            last_ident = None;
            last_end = None;
            continue;
        }

        // Идентификаторы / ключевые слова
        if is_ident_start(c) {
            let mut ident = String::new();
            ident.push(c);
            out.push(c);
            while let Some(peek) = chars.peek() {
                if is_ident_continue(*peek) {
                    ident.push(*peek);
                    out.push(*peek);
                    chars.next();
                } else {
                    break;
                }
            }
            if block_keywords.contains(&ident.as_str()) {
                block_header = true;
            }
            last_ident = Some(ident);
            last_end = Some(LastTokenEnd::Identifier);
            continue;
        }

        // Числа (упрощённо: цифры и точка)
        if c.is_ascii_digit() {
            out.push(c);
            while let Some(peek) = chars.peek() {
                if peek.is_ascii_digit() || *peek == '.' {
                    out.push(*peek);
                    chars.next();
                } else {
                    break;
                }
            }
            last_end = Some(LastTokenEnd::Number);
            last_ident = None;
            continue;
        }

        if !c.is_whitespace() {
            last_end = match c {
                ')' | ']' | '}' => Some(LastTokenEnd::CloseDelim),
                other => Some(LastTokenEnd::Other(other)),
            };
            last_ident = None;
        }
        out.push(c);
    }

    out
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
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
