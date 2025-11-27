use crate::ponos::span::Span;
use std::fmt;
use winnow::error::{ParserError, ErrorKind, FromExternalError};
use winnow::stream::Stream;

/// Тип ошибки парсинга
#[derive(Debug, Clone, PartialEq)]
pub enum ParseErrorKind {
    /// Неожиданный токен (ожидалось, найдено)
    UnexpectedToken {
        expected: Vec<String>,
        found: String,
    },
    /// Неожиданный конец файла
    UnexpectedEof,
    /// Неверный формат числа
    InvalidNumber(String),
    /// Неверный формат строки
    InvalidString(String),
    /// Неверный идентификатор
    InvalidIdentifier(String),
    /// Произвольная ошибка
    Custom(String),
}

/// Ошибка парсинга Ponos
#[derive(Debug, Clone)]
pub struct PonosParseError {
    pub kind: ParseErrorKind,
    pub span: Span,
    pub context: Vec<String>,
}

impl PonosParseError {
    pub fn new(kind: ParseErrorKind, span: Span) -> Self {
        PonosParseError {
            kind,
            span,
            context: Vec::new(),
        }
    }

    pub fn with_context(mut self, ctx: String) -> Self {
        self.context.push(ctx);
        self
    }

    /// Форматирует ошибку с подсветкой исходного кода
    pub fn format(&self, source: &str, filename: &str) -> String {
        let (start_loc, end_loc) = self.span.to_location(source);

        let mut output = String::new();

        // Заголовок ошибки
        output.push_str(&format!(
            "Ошибка: {} в {}:{}:{}\n",
            self.kind.message(),
            filename,
            start_loc.line + 1,
            start_loc.column + 1
        ));

        // Контекст
        for ctx in &self.context {
            output.push_str(&format!("  в {}\n", ctx));
        }

        // Исходный код с подсветкой
        let lines: Vec<&str> = source.lines().collect();
        if start_loc.line < lines.len() {
            let error_line = lines[start_loc.line];

            output.push_str(&format!("\n{:4} | {}\n", start_loc.line + 1, error_line));
            output.push_str(&format!("     | {}", " ".repeat(start_loc.column)));

            let underline_len = if start_loc.line == end_loc.line {
                (end_loc.column - start_loc.column).max(1)
            } else {
                error_line.len() - start_loc.column
            };

            output.push_str(&format!("{}\n", "^".repeat(underline_len)));
        }

        // Подсказка
        if let Some(hint) = self.kind.hint() {
            output.push_str(&format!("\nПодсказка: {}\n", hint));
        }

        output
    }
}

impl ParseErrorKind {
    fn message(&self) -> String {
        match self {
            ParseErrorKind::UnexpectedToken { expected, found } => {
                if expected.is_empty() {
                    format!("Неожиданный токен '{}'", found)
                } else {
                    format!(
                        "Неожиданный токен '{}'. Ожидалось: {}",
                        found,
                        expected.join(", ")
                    )
                }
            }
            ParseErrorKind::UnexpectedEof => {
                "Неожиданный конец файла".to_string()
            }
            ParseErrorKind::InvalidNumber(s) => {
                format!("Неверный формат числа: '{}'", s)
            }
            ParseErrorKind::InvalidString(s) => {
                format!("Неверный формат строки: '{}'", s)
            }
            ParseErrorKind::InvalidIdentifier(s) => {
                format!("Неверный идентификатор: '{}'", s)
            }
            ParseErrorKind::Custom(msg) => msg.clone(),
        }
    }

    fn hint(&self) -> Option<String> {
        match self {
            ParseErrorKind::UnexpectedToken { expected, .. }
                if expected.contains(&";".to_string()) => {
                Some("Возможно, вы забыли поставить точку с запятой?".to_string())
            }
            ParseErrorKind::UnexpectedToken { expected, .. }
                if expected.contains(&"конец".to_string()) => {
                Some("Возможно, вы забыли закрыть блок словом 'конец'?".to_string())
            }
            ParseErrorKind::UnexpectedEof => {
                Some("Файл закончился раньше времени. Проверьте, все ли блоки закрыты.".to_string())
            }
            _ => None,
        }
    }
}

impl fmt::Display for PonosParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind.message())
    }
}

impl std::error::Error for PonosParseError {}

// Реализация ParserError для интеграции с winnow
impl<I: Stream> ParserError<I> for PonosParseError {
    fn from_error_kind(_input: &I, _kind: ErrorKind) -> Self {
        PonosParseError::new(
            ParseErrorKind::Custom("Ошибка парсинга".to_string()),
            Span::default(),
        )
    }

    fn append(self, _input: &I, _token_start: &<I as Stream>::Checkpoint, _kind: ErrorKind) -> Self {
        self
    }
}

impl<I: Stream, E: std::error::Error + Send + Sync + 'static> FromExternalError<I, E> for PonosParseError {
    fn from_external_error(_input: &I, _kind: ErrorKind, _e: E) -> Self {
        PonosParseError::new(
            ParseErrorKind::Custom("Внешняя ошибка".to_string()),
            Span::default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_format() {
        let source = "пер x = ;\nпер y = 10;";
        let error = PonosParseError::new(
            ParseErrorKind::UnexpectedToken {
                expected: vec!["выражение".to_string()],
                found: ";".to_string(),
            },
            Span::new(8, 9),
        ).with_context("объявление переменной".to_string());

        let formatted = error.format(source, "<test>");
        assert!(formatted.contains("Ошибка:"));
        assert!(formatted.contains("пер x = ;"));
        assert!(formatted.contains("^"));
    }
}
