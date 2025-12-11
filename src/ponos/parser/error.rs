use crate::ponos::span::Span;
use std::fmt;
use winnow::error::{ErrorKind, FromExternalError, ParserError};
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

        // Цвета ANSI
        let red = "\x1b[31m";
        let yellow = "\x1b[33m";
        let blue = "\x1b[34m";
        let bold = "\x1b[1m";
        let reset = "\x1b[0m";
        let gray = "\x1b[90m";

        // Заголовок ошибки с цветом
        output.push_str(&format!(
            "{}{}Ошибка:{} {} в {}{}:{}:{}{}\n",
            bold,
            red,
            reset,
            self.kind.message(),
            blue,
            filename,
            start_loc.line + 1,
            start_loc.column + 1,
            reset
        ));

        // Контекст
        for ctx in &self.context {
            output.push_str(&format!("{}  в {}{}\n", gray, ctx, reset));
        }

        // Исходный код с подсветкой и контекстом
        let lines: Vec<&str> = source.lines().collect();
        if start_loc.line < lines.len() {
            output.push_str("\n");

            // Показываем 2 строки до ошибки для контекста
            let context_start = start_loc.line.saturating_sub(2);
            for i in context_start..start_loc.line {
                if i < lines.len() {
                    output.push_str(&format!(
                        "{}{:4} |{} {}\n",
                        gray,
                        i + 1,
                        reset,
                        lines[i]
                    ));
                }
            }

            // Строка с ошибкой
            let error_line = lines[start_loc.line];
            output.push_str(&format!(
                "{}{:4} |{} {}\n",
                blue,
                start_loc.line + 1,
                reset,
                error_line
            ));

            // Подчеркивание ошибки
            let error_line_char_count = error_line.chars().count();
            let underline_len = if start_loc.line == end_loc.line {
                (end_loc.column - start_loc.column).max(1)
            } else {
                error_line_char_count.saturating_sub(start_loc.column)
            };

            output.push_str(&format!(
                "{}     |{} {}{}{}{}",
                blue,
                reset,
                " ".repeat(start_loc.column),
                red,
                "^".repeat(underline_len),
                reset
            ));

            // Добавляем текст что именно неправильно
            let found_text = if start_loc.column < error_line_char_count {
                let start_byte = error_line
                    .char_indices()
                    .nth(start_loc.column)
                    .map(|(idx, _)| idx)
                    .unwrap_or(error_line.len());
                let end_byte = error_line
                    .char_indices()
                    .nth(start_loc.column + underline_len)
                    .map(|(idx, _)| idx)
                    .unwrap_or(error_line.len());
                &error_line[start_byte..end_byte.min(error_line.len())]
            } else {
                ""
            };

            if !found_text.is_empty() && found_text.trim().len() > 0 {
                output.push_str(&format!(" {}{}{}", red, found_text, reset));
            }
            output.push_str("\n");

            // Показываем 1 строку после ошибки для контекста
            if start_loc.line + 1 < lines.len() {
                output.push_str(&format!(
                    "{}{:4} |{} {}\n",
                    gray,
                    start_loc.line + 2,
                    reset,
                    lines[start_loc.line + 1]
                ));
            }
        }

        // Подсказка с цветом
        if let Some(hint) = self.kind.hint() {
            output.push_str(&format!("\n{}{}💡 Подсказка:{} {}\n", bold, yellow, reset, hint));
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
            ParseErrorKind::UnexpectedEof => "Неожиданный конец файла".to_string(),
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
            ParseErrorKind::UnexpectedToken { expected, found } => {
                // Подсказки для точки с запятой
                if expected.contains(&";".to_string()) {
                    return Some("Возможно, вы забыли поставить точку с запятой? Операторы 'пер', 'возврат', 'исключение' и выражения должны заканчиваться на ';'".to_string());
                }

                // Подсказки для 'конец'
                if expected.contains(&"конец".to_string()) {
                    return Some("Возможно, вы забыли закрыть блок словом 'конец'? Блоки 'функ', 'класс', 'если', 'пока' должны заканчиваться на 'конец'".to_string());
                }

                // Подсказка для скобок
                if expected.contains(&")".to_string()) {
                    return Some("Возможно, вы забыли закрыть скобку ')'?".to_string());
                }
                if expected.contains(&"]".to_string()) {
                    return Some("Возможно, вы забыли закрыть скобку ']'?".to_string());
                }
                if expected.contains(&"}".to_string()) {
                    return Some("Возможно, вы забыли закрыть скобку '}'?".to_string());
                }

                // Подсказка для оператора присваивания
                if expected.contains(&"=".to_string()) {
                    return Some("Возможно, вы забыли оператор присваивания '='?".to_string());
                }

                // Подсказка для двоеточия (типы, словари)
                if expected.contains(&":".to_string()) {
                    return Some("Возможно, вы забыли двоеточие ':'? Оно нужно для типов или пар в словарях.".to_string());
                }

                // Подсказка если нашли 'иначе' вместо 'иначе если'
                if found.starts_with("иначе") && expected.contains(&"конец".to_string()) {
                    return Some("Возможно, вы хотели написать 'иначе если' вместо просто 'иначе'?".to_string());
                }

                // Подсказка для незакрытых строк
                if found.contains("\"") || found.contains("'") {
                    return Some("Возможно, у вас незакрытая строка? Проверьте кавычки.".to_string());
                }

                None
            }
            ParseErrorKind::UnexpectedEof => {
                Some("Файл закончился раньше времени. Проверьте, все ли блоки закрыты словом 'конец', все ли строки закрыты кавычками, и все ли скобки закрыты.".to_string())
            }
            ParseErrorKind::InvalidNumber(num) => {
                Some(format!("Проверьте формат числа '{}'. Числа должны быть в формате: 42 или 3.14", num))
            }
            ParseErrorKind::InvalidString(_) => {
                Some("Строки должны быть заключены в двойные кавычки (\"). Используйте \\ для экранирования: \\n, \\t, \\\"".to_string())
            }
            ParseErrorKind::InvalidIdentifier(id) => {
                Some(format!("Идентификатор '{}' недопустим. Идентификаторы должны начинаться с буквы или _, и содержать только буквы, цифры и _", id))
            }
            ParseErrorKind::Custom(_) => None,
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

    fn append(
        self,
        _input: &I,
        _token_start: &<I as Stream>::Checkpoint,
        _kind: ErrorKind,
    ) -> Self {
        self
    }
}

impl<I: Stream, E: std::error::Error + Send + Sync + 'static> FromExternalError<I, E>
    for PonosParseError
{
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
        )
        .with_context("объявление переменной".to_string());

        let formatted = error.format(source, "<test>");
        assert!(formatted.contains("Ошибка:"));
        assert!(formatted.contains("пер x = ;"));
        assert!(formatted.contains("^"));
    }
}
