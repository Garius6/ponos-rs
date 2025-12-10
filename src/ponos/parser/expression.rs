use winnow::prelude::*;
use winnow::stream::Stream;
use winnow::combinator::{alt, separated, delimited};
use crate::ponos::ast::*;
use crate::ponos::span::Span;
use crate::ponos::parser::combinator::{Input, PResult, char_};
use crate::ponos::parser::error::{PonosParseError, ParseErrorKind};
use crate::ponos::parser::lexer::{
    parse_number, parse_string, parse_bool, parse_identifier,
    keyword_this, keyword_super, keyword_func, keyword_end,
    keyword_and, keyword_or, skip_ws_and_comments
};

/// Главная функция парсинга выражений
pub fn parse_expression<'a>(input: &mut Input<'a>) -> PResult<'a, Expression> {
    parse_binary_expression(input, 0)
}

/// Парсит бинарные выражения с учетом приоритета (Pratt parsing)
fn parse_binary_expression<'a>(input: &mut Input<'a>, min_precedence: u8) -> PResult<'a, Expression> {
    skip_ws_and_comments(input)?;
    let mut left = parse_unary_expression(input)?;

    loop {
        skip_ws_and_comments(input)?;

        // Сохраняем checkpoint для возможности отката
        let checkpoint = input.checkpoint();

        // Пытаемся спарсить бинарный оператор
        let op_result = parse_binary_operator(input);
        if op_result.is_err() {
            break;
        }

        let (operator, op_span) = op_result.unwrap();
        let precedence = operator.precedence();

        if precedence < min_precedence {
            // Откатываемся, так как приоритет слишком низкий
            input.reset(&checkpoint);
            break;
        }

        skip_ws_and_comments(input)?;

        // Парсим правую часть с учетом ассоциативности
        let next_min_prec = if operator.is_left_associative() {
            precedence + 1
        } else {
            precedence
        };

        let right = parse_binary_expression(input, next_min_prec)?;

        let span = Span::new(
            left.span().start,
            right.span().end,
        );

        left = Expression::Binary(Box::new(BinaryExpr {
            left,
            operator,
            right,
            span,
        }));
    }

    Ok(left)
}

/// Парсит бинарный оператор
fn parse_binary_operator<'a>(input: &mut Input<'a>) -> PResult<'a, (BinaryOperator, Span)> {
    let start = input.len();

    let op = alt((
        // Логические операторы (ключевые слова)
        keyword_and.map(|_| BinaryOperator::And),
        keyword_or.map(|_| BinaryOperator::Or),

        // Сравнения (двухсимвольные сначала!)
        "==".map(|_| BinaryOperator::Equal),
        "!=".map(|_| BinaryOperator::NotEqual),
        "<=".map(|_| BinaryOperator::LessEqual),
        ">=".map(|_| BinaryOperator::GreaterEqual),

        // Односимвольные операторы
        "<".map(|_| BinaryOperator::Less),
        ">".map(|_| BinaryOperator::Greater),
        "+".map(|_| BinaryOperator::Add),
        "-".map(|_| BinaryOperator::Subtract),
        "*".map(|_| BinaryOperator::Multiply),
        "/".map(|_| BinaryOperator::Divide),
        "%".map(|_| BinaryOperator::Modulo),
    )).parse_next(input)?;

    let end = input.len();
    let span = Span::new(start - end, start);

    Ok((op, span))
}

/// Парсит унарные выражения
fn parse_unary_expression<'a>(input: &mut Input<'a>) -> PResult<'a, Expression> {
    skip_ws_and_comments(input)?;

    let start = input.len();

    // Сохраняем checkpoint для backtrack
    let saved = input.checkpoint();

    // Пытаемся спарсить унарный оператор
    let op_result: PResult<'a, UnaryOperator> = alt((
        "-".map(|_| UnaryOperator::Negate),
        "!".map(|_| UnaryOperator::Not),
    )).parse_next(input);

    if let Ok(operator) = op_result {
        skip_ws_and_comments(input)?;
        let operand = parse_unary_expression(input)?;
        let end = input.len();
        let span = Span::new(start - end, start);

        Ok(Expression::Unary(Box::new(UnaryExpr {
            operator,
            operand,
            span,
        })))
    } else {
        // Нет унарного оператора, восстанавливаем позицию и парсим постфиксное выражение
        input.reset(&saved);
        parse_postfix_expression(input)
    }
}

/// Парсит постфиксные выражения (вызовы функций, доступ к полям)
fn parse_postfix_expression<'a>(input: &mut Input<'a>) -> PResult<'a, Expression> {
    let mut expr = parse_primary_expression(input)?;

    loop {
        skip_ws_and_comments(input)?;

        // Пытаемся спарсить постфиксный оператор
        let saved = input.checkpoint();

        // Вызов функции: expr(args)
        if char_('(').parse_next(input).is_ok() {
            skip_ws_and_comments(input)?;

            // Парсим аргументы
            let arguments = if char_(')').parse_next(input).is_ok() {
                // Нет аргументов
                Vec::new()
            } else {
                input.reset(&saved);
                *input = &input[1..]; // skip '('
                skip_ws_and_comments(input)?;

                let args = separated(
                    0..,
                    parse_expression,
                    (skip_ws_and_comments, char_(','), skip_ws_and_comments)
                ).parse_next(input)?;

                skip_ws_and_comments(input)?;
                char_(')').parse_next(input)?;

                args
            };

            let span = Span::new(expr.span().start, input.len());

            expr = Expression::Call(Box::new(CallExpr {
                callee: expr,
                arguments,
                span,
            }));
            continue;
        }

        input.reset(&saved);

        // Доступ к полю: expr.field
        if char_('.').parse_next(input).is_ok() {
            skip_ws_and_comments(input)?;
            let field = parse_identifier(input)?.to_string();
            let span = Span::new(expr.span().start, input.len());

            expr = Expression::FieldAccess(Box::new(FieldAccessExpr {
                object: expr,
                field,
                span,
            }));
            continue;
        }

        input.reset(&saved);

        // Индексирование: expr[index] или срез: expr[start:end]
        if char_('[').parse_next(input).is_ok() {
            skip_ws_and_comments(input)?;

            // Проверяем, не срез ли это (начинается с :)
            let checkpoint = input.checkpoint();
            let first_expr = if char_(':').parse_next(input).is_ok() {
                // Это [:end] - срез без начала
                None
            } else {
                input.reset(&checkpoint);
                Some(Box::new(parse_expression(input)?))
            };

            skip_ws_and_comments(input)?;

            // Проверяем наличие :
            let is_slice = char_(':').parse_next(input).is_ok();

            if is_slice {
                // Это срез [start:end]
                skip_ws_and_comments(input)?;

                // Проверяем, есть ли второе выражение
                let checkpoint2 = input.checkpoint();
                let second_expr = if char_(']').parse_next(input).is_ok() {
                    // Это [start:] - срез без конца
                    None
                } else {
                    input.reset(&checkpoint2);
                    let expr = parse_expression(input)?;
                    skip_ws_and_comments(input)?;
                    char_(']').parse_next(input)?;
                    Some(Box::new(expr))
                };

                // Создаем Range выражение
                let range_span = Span::new(0, 0);  // TODO: правильный span
                let range = Expression::Range(Box::new(crate::ponos::ast::RangeExpr {
                    start: first_expr,
                    end: second_expr,
                    span: range_span,
                }));

                let span = Span::new(expr.span().start, input.len());
                expr = Expression::Index(Box::new(crate::ponos::ast::IndexExpr {
                    object: expr,
                    index: range,
                    span,
                }));
            } else {
                // Обычное индексирование [index]
                skip_ws_and_comments(input)?;
                char_(']').parse_next(input)?;

                let span = Span::new(expr.span().start, input.len());
                expr = Expression::Index(Box::new(crate::ponos::ast::IndexExpr {
                    object: expr,
                    index: *first_expr.unwrap(),
                    span,
                }));
            }
            continue;
        }

        input.reset(&saved);

        // Больше нет постфиксных операторов
        break;
    }

    Ok(expr)
}

/// Парсит примитивные выражения
fn parse_primary_expression<'a>(input: &mut Input<'a>) -> PResult<'a, Expression> {
    skip_ws_and_comments(input)?;

    alt((
        parse_number_expr,
        parse_string_expr,
        parse_bool_expr,
        parse_this_expr,
        parse_super_expr,
        parse_lambda_expr,
        parse_parenthesized_expr,
        parse_identifier_expr,
    )).parse_next(input)
}

fn parse_number_expr<'a>(input: &mut Input<'a>) -> PResult<'a, Expression> {
    let start = input.len();
    let num = parse_number(input)?;
    let end = input.len();
    let span = Span::new(start - end, start);
    Ok(Expression::Number(num, span))
}

fn parse_string_expr<'a>(input: &mut Input<'a>) -> PResult<'a, Expression> {
    let start = input.len();
    let s = parse_string(input)?;
    let end = input.len();
    let span = Span::new(start - end, start);
    Ok(Expression::String(s, span))
}

fn parse_bool_expr<'a>(input: &mut Input<'a>) -> PResult<'a, Expression> {
    let start = input.len();
    let b = parse_bool(input)?;
    let end = input.len();
    let span = Span::new(start - end, start);
    Ok(Expression::Boolean(b, span))
}

fn parse_this_expr<'a>(input: &mut Input<'a>) -> PResult<'a, Expression> {
    let start = input.len();
    keyword_this.parse_next(input)?;
    let end = input.len();
    let span = Span::new(start - end, start);
    Ok(Expression::This(span))
}

fn parse_super_expr<'a>(input: &mut Input<'a>) -> PResult<'a, Expression> {
    let start = input.len();
    keyword_super.parse_next(input)?;
    skip_ws_and_comments(input)?;
    char_('.').parse_next(input)?;
    skip_ws_and_comments(input)?;
    let method = parse_identifier(input)?.to_string();
    let end = input.len();
    let span = Span::new(start - end, start);
    Ok(Expression::Super(method, span))
}

fn parse_identifier_expr<'a>(input: &mut Input<'a>) -> PResult<'a, Expression> {
    let start = input.len();
    let id = parse_identifier(input)?.to_string();
    let end = input.len();
    let span = Span::new(start - end, start);
    Ok(Expression::Identifier(id, span))
}

fn parse_parenthesized_expr<'a>(input: &mut Input<'a>) -> PResult<'a, Expression> {
    delimited(
        char_('('),
        (skip_ws_and_comments, parse_expression, skip_ws_and_comments),
        char_(')')
    ).parse_next(input).map(|(_, expr, _)| expr)
}

fn parse_lambda_expr<'a>(input: &mut Input<'a>) -> PResult<'a, Expression> {
    let start = input.len();

    // функ (params) statements конец
    keyword_func.parse_next(input)?;
    skip_ws_and_comments(input)?;

    // Парсим параметры
    char_('(').parse_next(input)?;
    skip_ws_and_comments(input)?;

    let params = if char_(')').parse_next(input).is_ok() {
        Vec::new()
    } else {
        let params: Vec<Parameter> = separated(
            0..,
            parse_parameter,
            (skip_ws_and_comments, char_(','), skip_ws_and_comments)
        ).parse_next(input)?;

        skip_ws_and_comments(input)?;
        char_(')').parse_next(input)?;

        params
    };

    skip_ws_and_comments(input)?;

    // Опциональный тип возврата: ": тип"
    let _return_type = if char_(':').parse_next(input).is_ok() {
        skip_ws_and_comments(input)?;
        Some(parse_identifier(input)?.to_string())
    } else {
        None
    };

    skip_ws_and_comments(input)?;

    // Парсим тело (statements до конец)
    let mut body = Vec::new();
    loop {
        skip_ws_and_comments(input)?;

        // Проверяем, не конец ли блока
        let saved = input.checkpoint();
        if keyword_end.parse_next(input).is_ok() {
            break;
        }
        input.reset(&saved);

        // Парсим оператор
        use crate::ponos::parser::statement;
        let stmt = statement::parse_statement(input)?;
        body.push(stmt);
    }

    let end = input.len();
    let span = Span::new(start - end, start);

    Ok(Expression::Lambda(Box::new(LambdaExpr {
        params,
        body,
        span,
    })))
}

fn parse_parameter<'a>(input: &mut Input<'a>) -> PResult<'a, Parameter> {
    let start = input.len();
    let name = parse_identifier(input)?.to_string();

    skip_ws_and_comments(input)?;

    // Опциональная аннотация типа
    let type_annotation = if char_(':').parse_next(input).is_ok() {
        skip_ws_and_comments(input)?;
        Some(parse_identifier(input)?.to_string())
    } else {
        None
    };

    let end = input.len();
    let span = Span::new(start - end, start);

    Ok(Parameter {
        name,
        type_annotation,
        span,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_number_expr() {
        let mut input = "42";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Number(n, _) => assert_eq!(n, 42.0),
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_parse_string_expr() {
        let mut input = r#""hello""#;
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::String(s, _) => assert_eq!(s, "hello"),
            _ => panic!("Expected string"),
        }
    }

    #[test]
    fn test_parse_identifier() {
        let mut input = "x";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Identifier(id, _) => assert_eq!(id, "x"),
            _ => panic!("Expected identifier"),
        }
    }

    #[test]
    fn test_parse_binary_add() {
        let mut input = "2 + 3";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Binary(b) => {
                assert_eq!(b.operator, BinaryOperator::Add);
                match (&b.left, &b.right) {
                    (Expression::Number(l, _), Expression::Number(r, _)) => {
                        assert_eq!(*l, 2.0);
                        assert_eq!(*r, 3.0);
                    }
                    _ => panic!("Expected numbers"),
                }
            }
            _ => panic!("Expected binary expr"),
        }
    }

    #[test]
    fn test_parse_binary_precedence() {
        // 2 + 3 * 4 должно быть 2 + (3 * 4)
        let mut input = "2 + 3 * 4";
        let expr = parse_expression(&mut input).unwrap();

        match expr {
            Expression::Binary(b) => {
                assert_eq!(b.operator, BinaryOperator::Add);
                match (&b.left, &b.right) {
                    (Expression::Number(l, _), Expression::Binary(r)) => {
                        assert_eq!(*l, 2.0);
                        assert_eq!(r.operator, BinaryOperator::Multiply);
                    }
                    _ => panic!("Wrong structure"),
                }
            }
            _ => panic!("Expected binary expr"),
        }
    }

    #[test]
    fn test_parse_unary() {
        let mut input = "-5";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Unary(u) => {
                assert_eq!(u.operator, UnaryOperator::Negate);
                match &u.operand {
                    Expression::Number(n, _) => assert_eq!(*n, 5.0),
                    _ => panic!("Expected number"),
                }
            }
            _ => panic!("Expected unary expr"),
        }
    }

    #[test]
    fn test_parse_call() {
        let mut input = "foo()";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Call(c) => {
                match &c.callee {
                    Expression::Identifier(id, _) => assert_eq!(id, "foo"),
                    _ => panic!("Expected identifier"),
                }
                assert_eq!(c.arguments.len(), 0);
            }
            _ => panic!("Expected call expr"),
        }

        let mut input = "bar(1, 2)";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Call(c) => {
                assert_eq!(c.arguments.len(), 2);
            }
            _ => panic!("Expected call expr"),
        }
    }

    #[test]
    fn test_parse_field_access() {
        let mut input = "obj.field";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::FieldAccess(f) => {
                match &f.object {
                    Expression::Identifier(id, _) => assert_eq!(id, "obj"),
                    _ => panic!("Expected identifier"),
                }
                assert_eq!(f.field, "field");
            }
            _ => panic!("Expected field access"),
        }
    }

    #[test]
    fn test_parse_this() {
        let mut input = "это";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::This(_) => {}
            _ => panic!("Expected this"),
        }
    }

    #[test]
    fn test_parse_parenthesized() {
        let mut input = "(2 + 3) * 4";
        let expr = parse_expression(&mut input).unwrap();

        // Должно быть (2 + 3) * 4
        match expr {
            Expression::Binary(b) => {
                assert_eq!(b.operator, BinaryOperator::Multiply);
                match &b.left {
                    Expression::Binary(left_bin) => {
                        assert_eq!(left_bin.operator, BinaryOperator::Add);
                    }
                    _ => panic!("Expected binary in left"),
                }
            }
            _ => panic!("Expected binary expr"),
        }
    }

    #[test]
    fn test_parse_lambda() {
        let mut input = "функ(x, y) возврат x + y; конец";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Lambda(l) => {
                assert_eq!(l.params.len(), 2);
                assert_eq!(l.params[0].name, "x");
                assert_eq!(l.params[1].name, "y");
                assert_eq!(l.body.len(), 1);
            }
            _ => panic!("Expected lambda"),
        }
    }

    #[test]
    fn test_parse_lambda_with_typed_params() {
        let mut input = "функ(x: число, y: строка) возврат x; конец";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Lambda(l) => {
                assert_eq!(l.params.len(), 2);
                assert_eq!(l.params[0].name, "x");
                assert_eq!(l.params[0].type_annotation, Some("число".to_string()));
                assert_eq!(l.params[1].name, "y");
                assert_eq!(l.params[1].type_annotation, Some("строка".to_string()));
            }
            _ => panic!("Expected lambda"),
        }
    }

    #[test]
    fn test_parse_lambda_with_return_type() {
        let mut input = "функ(a: число, b: число): число возврат a + b; конец";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Lambda(l) => {
                assert_eq!(l.params.len(), 2);
                assert_eq!(l.params[0].name, "a");
                assert_eq!(l.params[0].type_annotation, Some("число".to_string()));
                assert_eq!(l.params[1].name, "b");
                assert_eq!(l.params[1].type_annotation, Some("число".to_string()));
                assert_eq!(l.body.len(), 1);
            }
            _ => panic!("Expected lambda"),
        }
    }

    #[test]
    fn test_parse_lambda_no_params() {
        let mut input = "функ() возврат 42; конец";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Lambda(l) => {
                assert_eq!(l.params.len(), 0);
                assert_eq!(l.body.len(), 1);
            }
            _ => panic!("Expected lambda"),
        }
    }

    #[test]
    fn test_parse_super() {
        let mut input = "родитель.метод";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Super(method, _) => {
                assert_eq!(method, "метод");
            }
            _ => panic!("Expected super"),
        }
    }

    #[test]
    fn test_parse_logical_and() {
        let mut input = "истина и ложь";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Binary(b) => {
                assert_eq!(b.operator, BinaryOperator::And);
            }
            _ => panic!("Expected binary and"),
        }
    }

    #[test]
    fn test_parse_logical_or() {
        let mut input = "истина или ложь";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Binary(b) => {
                assert_eq!(b.operator, BinaryOperator::Or);
            }
            _ => panic!("Expected binary or"),
        }
    }

    #[test]
    fn test_parse_logical_not() {
        let mut input = "!истина";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Unary(u) => {
                assert_eq!(u.operator, UnaryOperator::Not);
            }
            _ => panic!("Expected unary not"),
        }
    }

    #[test]
    fn test_parse_comparison_equal() {
        let mut input = "x == y";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Binary(b) => {
                assert_eq!(b.operator, BinaryOperator::Equal);
            }
            _ => panic!("Expected binary equal"),
        }
    }

    #[test]
    fn test_parse_comparison_not_equal() {
        let mut input = "x != y";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Binary(b) => {
                assert_eq!(b.operator, BinaryOperator::NotEqual);
            }
            _ => panic!("Expected binary not equal"),
        }
    }

    #[test]
    fn test_parse_comparison_less() {
        let mut input = "x < y";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Binary(b) => {
                assert_eq!(b.operator, BinaryOperator::Less);
            }
            _ => panic!("Expected binary less"),
        }
    }

    #[test]
    fn test_parse_comparison_less_equal() {
        let mut input = "x <= y";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Binary(b) => {
                assert_eq!(b.operator, BinaryOperator::LessEqual);
            }
            _ => panic!("Expected binary less equal"),
        }
    }

    #[test]
    fn test_parse_comparison_greater() {
        let mut input = "x > y";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Binary(b) => {
                assert_eq!(b.operator, BinaryOperator::Greater);
            }
            _ => panic!("Expected binary greater"),
        }
    }

    #[test]
    fn test_parse_comparison_greater_equal() {
        let mut input = "x >= y";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Binary(b) => {
                assert_eq!(b.operator, BinaryOperator::GreaterEqual);
            }
            _ => panic!("Expected binary greater equal"),
        }
    }

    #[test]
    fn test_parse_subtract() {
        let mut input = "10 - 5";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Binary(b) => {
                assert_eq!(b.operator, BinaryOperator::Subtract);
            }
            _ => panic!("Expected binary subtract"),
        }
    }

    #[test]
    fn test_parse_multiply() {
        let mut input = "3 * 4";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Binary(b) => {
                assert_eq!(b.operator, BinaryOperator::Multiply);
            }
            _ => panic!("Expected binary multiply"),
        }
    }

    #[test]
    fn test_parse_divide() {
        let mut input = "10 / 2";
        let expr = parse_expression(&mut input).unwrap();
        match expr {
            Expression::Binary(b) => {
                assert_eq!(b.operator, BinaryOperator::Divide);
            }
            _ => panic!("Expected binary divide"),
        }
    }
}
