use winnow::prelude::*;
use winnow::token::take_while;
use winnow::combinator::{alt, preceded};
use crate::ponos::parser::combinator::{Input, PResult, keyword, identifier as ident_parser};
use crate::ponos::parser::error::{PonosParseError, ParseErrorKind};
use crate::ponos::span::Span;

/// Парсит число (целое или с плавающей точкой)
pub fn parse_number<'a>(input: &mut Input<'a>) -> PResult<'a, f64> {
    let start_len = input.len();

    // Парсим целую часть
    let int_part = take_while(1.., |c: char| c.is_ascii_digit()).parse_next(input)?;

    // Опционально парсим дробную часть
    let frac_part: Option<&str> = if input.starts_with('.') {
        *input = &input[1..]; // consume '.'
        take_while::<_, _, PonosParseError>(1.., |c: char| c.is_ascii_digit()).parse_next(input).ok()
    } else {
        None
    };

    let end_len = input.len();
    let _span = Span::new(start_len - end_len, start_len);

    // Собираем строку и парсим число
    let num_str = if let Some(frac) = frac_part {
        format!("{}.{}", int_part, frac)
    } else {
        int_part.to_string()
    };

    num_str.parse::<f64>().map_err(|_| {
        winnow::error::ErrMode::Backtrack(PonosParseError::new(
            ParseErrorKind::InvalidNumber(num_str),
            Span::new(0, 10),
        ))
    })
}

/// Парсит строковый литерал с escape-последовательностями
pub fn parse_string<'a>(input: &mut Input<'a>) -> PResult<'a, String> {
    let start_len = input.len();

    // Ожидаем открывающую кавычку
    if !input.starts_with('"') {
        return Err(winnow::error::ErrMode::Backtrack(PonosParseError::new(
            ParseErrorKind::UnexpectedToken {
                expected: vec!["\"".to_string()],
                found: input.chars().next().map(|c| c.to_string()).unwrap_or_else(|| "EOF".to_string()),
            },
            Span::new(0, 1),
        )));
    }
    *input = &input[1..]; // consume '"'

    let mut result = String::new();
    let mut escaped = false;

    while let Some(c) = input.chars().next() {
        if escaped {
            // Обрабатываем escape-последовательность
            match c {
                'n' => result.push('\n'),
                't' => result.push('\t'),
                'r' => result.push('\r'),
                '\\' => result.push('\\'),
                '"' => result.push('"'),
                '\'' => result.push('\''),
                '0' => result.push('\0'),
                _ => {
                    // Неизвестная escape-последовательность
                    return Err(winnow::error::ErrMode::Backtrack(PonosParseError::new(
                        ParseErrorKind::InvalidString(format!("Неизвестная escape-последовательность: \\{}", c)),
                        Span::new(0, 2),
                    )));
                }
            }
            escaped = false;
            *input = &input[c.len_utf8()..];
        } else if c == '\\' {
            escaped = true;
            *input = &input[1..];
        } else if c == '"' {
            // Закрывающая кавычка
            *input = &input[1..];
            return Ok(result);
        } else if c == '\n' {
            // Строка не может содержать незакрытый перевод строки
            return Err(winnow::error::ErrMode::Backtrack(PonosParseError::new(
                ParseErrorKind::InvalidString("Незакрытая строка".to_string()),
                Span::new(0, start_len - input.len()),
            )));
        } else {
            result.push(c);
            *input = &input[c.len_utf8()..];
        }
    }

    // Если мы дошли до конца без закрывающей кавычки
    Err(winnow::error::ErrMode::Backtrack(PonosParseError::new(
        ParseErrorKind::InvalidString("Незакрытая строка".to_string()),
        Span::new(0, start_len),
    )))
}

/// Парсит булево значение
pub fn parse_bool<'a>(input: &mut Input<'a>) -> PResult<'a, bool> {
    alt((
        keyword("истина").map(|_| true),
        keyword("ложь").map(|_| false),
    )).parse_next(input)
}

/// Парсит идентификатор (обертка для удобства)
pub fn parse_identifier<'a>(input: &mut Input<'a>) -> PResult<'a, &'a str> {
    ident_parser(input)
}

/// Парсит однострочный комментарий
pub fn line_comment<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    if !input.starts_with("//") {
        return Err(winnow::error::ErrMode::Backtrack(PonosParseError::new(
            ParseErrorKind::UnexpectedToken {
                expected: vec!["//".to_string()],
                found: input.chars().take(2).collect(),
            },
            Span::new(0, 2),
        )));
    }
    *input = &input[2..];

    // Читаем до конца строки
    while let Some(c) = input.chars().next() {
        if c == '\n' {
            break;
        }
        *input = &input[c.len_utf8()..];
    }

    Ok(())
}

/// Парсит многострочный комментарий
pub fn block_comment<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    if !input.starts_with("/*") {
        return Err(winnow::error::ErrMode::Backtrack(PonosParseError::new(
            ParseErrorKind::UnexpectedToken {
                expected: vec!["/*".to_string()],
                found: input.chars().take(2).collect(),
            },
            Span::new(0, 2),
        )));
    }
    *input = &input[2..];

    // Читаем до */
    while !input.is_empty() {
        if input.starts_with("*/") {
            *input = &input[2..];
            return Ok(());
        }
        let c = input.chars().next().unwrap();
        *input = &input[c.len_utf8()..];
    }

    // Незакрытый комментарий
    Err(winnow::error::ErrMode::Backtrack(PonosParseError::new(
        ParseErrorKind::Custom("Незакрытый многострочный комментарий".to_string()),
        Span::new(0, 2),
    )))
}

/// Пропускает пробелы и комментарии
pub fn skip_ws_and_comments<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    loop {
        let before = input.len();

        // Пропускаем пробелы
        take_while::<_, _, PonosParseError>(0.., |c: char| c.is_whitespace()).parse_next(input)?;

        // Пытаемся пропустить комментарий
        let _ = alt((line_comment, block_comment)).parse_next(input);

        // Если ничего не изменилось, выходим
        if input.len() == before {
            break;
        }
    }
    Ok(())
}

// Ключевые слова

pub fn keyword_var<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("пер").parse_next(input)
}

pub fn keyword_func<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("функ").parse_next(input)
}

pub fn keyword_end<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("конец").parse_next(input)
}

pub fn keyword_class<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("класс").parse_next(input)
}

pub fn keyword_interface<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("интерфейс").parse_next(input)
}

pub fn keyword_annotation<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("аннотация").parse_next(input)
}

pub fn keyword_if<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("если").parse_next(input)
}

pub fn keyword_else<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("иначе").parse_next(input)
}

pub fn keyword_while<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("пока").parse_next(input)
}

pub fn keyword_return<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("возврат").parse_next(input)
}

pub fn keyword_try<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("попытка").parse_next(input)
}

pub fn keyword_catch<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("перехват").parse_next(input)
}

pub fn keyword_throw<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("исключение").parse_next(input)
}

pub fn keyword_export<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("экспорт").parse_next(input)
}

pub fn keyword_this<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("это").parse_next(input)
}

pub fn keyword_super<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("родитель").parse_next(input)
}

pub fn keyword_extends<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("наследует").parse_next(input)
}

pub fn keyword_implements<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("реализует").parse_next(input)
}

pub fn keyword_use<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("использовать").parse_next(input)
}

pub fn keyword_show<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("показать").parse_next(input)
}

pub fn keyword_hide<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("скрыть").parse_next(input)
}

pub fn keyword_as<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("как").parse_next(input)
}

pub fn keyword_constructor<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("конструктор").parse_next(input)
}

pub fn keyword_and<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("и").parse_next(input)
}

pub fn keyword_or<'a>(input: &mut Input<'a>) -> PResult<'a, ()> {
    keyword("или").parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_number() {
        let mut input = "42";
        assert_eq!(parse_number(&mut input).unwrap(), 42.0);

        let mut input = "3.14";
        assert_eq!(parse_number(&mut input).unwrap(), 3.14);

        let mut input = "0";
        assert_eq!(parse_number(&mut input).unwrap(), 0.0);

        let mut input = "123.456";
        assert_eq!(parse_number(&mut input).unwrap(), 123.456);
    }

    #[test]
    fn test_parse_bool() {
        let mut input = "истина";
        assert_eq!(parse_bool(&mut input).unwrap(), true);

        let mut input = "ложь";
        assert_eq!(parse_bool(&mut input).unwrap(), false);
    }

    #[test]
    fn test_parse_string() {
        let mut input = r#""привет""#;
        assert_eq!(parse_string(&mut input).unwrap(), "привет");

        let mut input = r#""hello world""#;
        assert_eq!(parse_string(&mut input).unwrap(), "hello world");

        let mut input = r#""строка\nс\tэкранированием""#;
        assert_eq!(parse_string(&mut input).unwrap(), "строка\nс\tэкранированием");

        let mut input = r#""кавычки: \" и слеш: \\""#;
        assert_eq!(parse_string(&mut input).unwrap(), "кавычки: \" и слеш: \\");

        let mut input = r#""""#;  // пустая строка
        assert_eq!(parse_string(&mut input).unwrap(), "");
    }

    #[test]
    fn test_parse_string_errors() {
        let mut input = r#""незакрытая"#;
        assert!(parse_string(&mut input).is_err());

        let mut input = "\"строка\nс переносом\"";
        assert!(parse_string(&mut input).is_err());
    }

    #[test]
    fn test_parse_identifier() {
        let mut input = "переменная";
        assert_eq!(parse_identifier(&mut input).unwrap(), "переменная");

        let mut input = "x";
        assert_eq!(parse_identifier(&mut input).unwrap(), "x");

        let mut input = "_private";
        assert_eq!(parse_identifier(&mut input).unwrap(), "_private");

        let mut input = "переменная123";
        assert_eq!(parse_identifier(&mut input).unwrap(), "переменная123");
    }

    #[test]
    fn test_line_comment() {
        let mut input = "// это комментарий\nкод";
        line_comment(&mut input).unwrap();
        assert_eq!(input, "\nкод");

        let mut input = "// комментарий до конца";
        line_comment(&mut input).unwrap();
        assert_eq!(input, "");
    }

    #[test]
    fn test_block_comment() {
        let mut input = "/* комментарий */код";
        block_comment(&mut input).unwrap();
        assert_eq!(input, "код");

        let mut input = "/* многострочный\n комментарий\n */код";
        block_comment(&mut input).unwrap();
        assert_eq!(input, "код");
    }

    #[test]
    fn test_skip_ws_and_comments() {
        let mut input = "  \n\t  код";
        skip_ws_and_comments(&mut input).unwrap();
        assert_eq!(input, "код");

        let mut input = "  // комментарий\n  код";
        skip_ws_and_comments(&mut input).unwrap();
        assert_eq!(input, "код");

        let mut input = "  /* блок */  // строка\n  код";
        skip_ws_and_comments(&mut input).unwrap();
        assert_eq!(input, "код");
    }
}
