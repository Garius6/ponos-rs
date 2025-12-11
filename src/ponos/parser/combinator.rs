use crate::ponos::parser::error::{ParseErrorKind, PonosParseError};
use crate::ponos::span::Span;
use std::cell::Cell;
use winnow::prelude::*;
use winnow::stream::{AsChar, Stream};
use winnow::token::take_while;

/// Тип входных данных для парсера
pub type Input<'a> = &'a str;

/// Тип результата парсера
pub type PResult<'a, O> = Result<O, winnow::error::ErrMode<PonosParseError>>;

thread_local! {
    static SOURCE_LENGTH: Cell<usize> = Cell::new(0);
}

/// Устанавливает длину исходника для расчёта абсолютных позиций
pub fn set_source_length(len: usize) {
    SOURCE_LENGTH.with(|cell| cell.set(len));
}

pub fn ensure_source_length(len: usize) {
    SOURCE_LENGTH.with(|cell| {
        if cell.get() == 0 {
            cell.set(len);
        }
    });
}

fn source_length_or(default: usize) -> usize {
    SOURCE_LENGTH.with(|cell| {
        let current = cell.get();
        if current == 0 {
            cell.set(default);
            default
        } else {
            current
        }
    })
}

pub fn offset_from_remaining(remaining: usize) -> usize {
    let len = source_length_or(remaining);
    len.saturating_sub(remaining)
}

pub fn span_from_remaining(start_remaining: usize, end_remaining: usize) -> Span {
    let len = source_length_or(start_remaining);
    Span::new(len.saturating_sub(start_remaining), len.saturating_sub(end_remaining))
}

pub fn span_with_width(start_remaining: usize, width: usize) -> Span {
    let start = offset_from_remaining(start_remaining);
    Span::new(start, start.saturating_add(width))
}

/// Пропускает пробельные символы и комментарии
pub fn ws<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    take_while(0.., |c: char| c.is_whitespace()).parse_next(input)?;
    Ok(())
}

/// Парсит ключевое слово, за которым должна следовать граница слова
pub fn keyword<'a>(kw: &'static str) -> impl Parser<Input<'a>, (), PonosParseError> {
    move |input: &mut Input<'a>| {
        let start_remaining = input.len();
        let start = input.checkpoint();

        // Проверяем, что строка начинается с ключевого слова
        if !input.starts_with(kw) {
            return Err(winnow::error::ErrMode::Backtrack(PonosParseError::new(
                ParseErrorKind::UnexpectedToken {
                    expected: vec![kw.to_string()],
                    found: input.chars().take(kw.len()).collect(),
                },
                span_with_width(start_remaining, kw.len()),
            )));
        }

        // Продвигаем input
        *input = &input[kw.len()..];

        // Проверяем границу слова (следующий символ не должен быть буквой/цифрой)
        if let Some(next_char) = input.chars().next() {
            if next_char.is_alphanumeric() || next_char == '_' {
                input.reset(&start);
                return Err(winnow::error::ErrMode::Backtrack(PonosParseError::new(
                    ParseErrorKind::UnexpectedToken {
                        expected: vec![kw.to_string()],
                        found: format!("{}{}", kw, next_char),
                    },
                    span_with_width(start_remaining, kw.len() + next_char.len_utf8()),
                )));
            }
        }

        Ok(())
    }
}

/// Парсит идентификатор (поддерживает Unicode)
pub fn identifier<'a>(input: &mut Input<'a>) -> PResult<'a, &'a str> {
    let start_remaining = input.len();
    let start = *input;
    let mut consumed = 0;

    // Первый символ - буква или _
    let first = input.chars().next();
    match first {
        Some(c) if c.is_alphabetic() || c == '_' => {
            consumed += c.len_utf8();
            *input = &input[c.len_utf8()..];
        }
        Some(c) => {
            return Err(winnow::error::ErrMode::Backtrack(PonosParseError::new(
                ParseErrorKind::InvalidIdentifier(c.to_string()),
                span_with_width(start_remaining, c.len_utf8()),
            )));
        }
        None => {
            return Err(winnow::error::ErrMode::Backtrack(PonosParseError::new(
                ParseErrorKind::UnexpectedEof,
                span_with_width(start_remaining, 0),
            )));
        }
    }

    // Остальные символы - буквы, цифры или _
    while let Some(c) = input.chars().next() {
        if c.is_alphanumeric() || c == '_' {
            consumed += c.len_utf8();
            *input = &input[c.len_utf8()..];
        } else {
            break;
        }
    }

    Ok(&start[..consumed])
}

/// Парсит конкретный символ
pub fn char_<'a>(ch: char) -> impl Parser<Input<'a>, char, PonosParseError> {
    move |input: &mut Input<'a>| {
        let start_remaining = input.len();

        if let Some(first) = input.chars().next() {
            if first == ch {
                *input = &input[first.len_utf8()..];
                return Ok(first);
            }
        }

        Err(winnow::error::ErrMode::Backtrack(PonosParseError::new(
            ParseErrorKind::UnexpectedToken {
                expected: vec![ch.to_string()],
                found: input
                    .chars()
                    .next()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "EOF".to_string()),
            },
            span_with_width(start_remaining, ch.len_utf8()),
        )))
    }
}

/// Оборачивает парсер для отслеживания позиций (span)
pub fn spanned<'a, O, P>(mut parser: P) -> impl Parser<Input<'a>, (O, Span), PonosParseError>
where
    P: Parser<Input<'a>, O, PonosParseError>,
{
    move |input: &mut Input<'a>| {
        let start = input.len();
        let result = parser.parse_next(input)?;
        let end = input.len();

        let span = span_from_remaining(start, end);
        Ok((result, span))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws() {
        let mut input = "   hello";
        ws(&mut input).unwrap();
        assert_eq!(input, "hello");

        let mut input = "\n\t  world";
        ws(&mut input).unwrap();
        assert_eq!(input, "world");
    }

    #[test]
    fn test_keyword() {
        let mut input = "пер x";
        keyword("пер").parse_next(&mut input).unwrap();
        assert_eq!(input, " x");

        let mut input = "перx";
        let result = keyword("пер").parse_next(&mut input);
        assert!(result.is_err());
    }

    #[test]
    fn test_char() {
        let mut input = ";rest";
        let c = char_(';').parse_next(&mut input).unwrap();
        assert_eq!(c, ';');
        assert_eq!(input, "rest");

        let mut input = "xrest";
        let result = char_(';').parse_next(&mut input);
        assert!(result.is_err());
    }

    #[test]
    fn test_span_from_remaining_uses_absolute_offsets() {
        set_source_length(12);
        // Начало входа: осталось 12 байт, потребляем 5
        let span = span_from_remaining(12, 7);
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 5);

        // Смещаемся внутри строки: осталось 7 байт, потребляем ещё 2
        let span2 = span_from_remaining(7, 5);
        assert_eq!(span2.start, 5);
        assert_eq!(span2.end, 7);
    }
}
