use winnow::prelude::*;
use winnow::stream::Stream;
use winnow::combinator::{alt, separated};
use crate::ponos::ast::*;
use crate::ponos::span::Span;
use crate::ponos::parser::combinator::{Input, PResult, char_};
use crate::ponos::parser::lexer::{
    parse_identifier, keyword_var, keyword_func, keyword_end,
    keyword_if, keyword_else, keyword_while, keyword_return,
    keyword_export, skip_ws_and_comments
};
use crate::ponos::parser::expression::parse_expression;

/// Парсит оператор (выбирает подходящий парсер)
pub fn parse_statement<'a>(input: &mut Input<'a>) -> PResult<'a, Statement> {
    skip_ws_and_comments(input)?;

    alt((
        parse_import_statement,
        parse_class_declaration,
        parse_interface_declaration,
        parse_annotation_declaration,
        parse_var_statement,
        parse_function_declaration,
        parse_if_statement,
        parse_while_statement,
        parse_return_statement,
        parse_assignment_or_expression_statement,
    )).parse_next(input)
}

/// Парсит объявление переменной: [экспорт] пер identifier [: type] = expression ;
pub fn parse_var_statement<'a>(input: &mut Input<'a>) -> PResult<'a, Statement> {
    let start = input.len();

    // Опциональное ключевое слово экспорт
    let is_exported = if keyword_export(input).is_ok() {
        skip_ws_and_comments(input)?;
        true
    } else {
        false
    };

    keyword_var(input)?;
    skip_ws_and_comments(input)?;

    let name = parse_identifier(input)?.to_string();
    skip_ws_and_comments(input)?;

    // Опциональная аннотация типа
    let type_annotation = if char_(':').parse_next(input).is_ok() {
        skip_ws_and_comments(input)?;
        Some(parse_identifier(input)?.to_string())
    } else {
        None
    };

    skip_ws_and_comments(input)?;

    // Инициализация (опциональная)
    let initializer = if char_('=').parse_next(input).is_ok() {
        skip_ws_and_comments(input)?;
        Some(parse_expression(input)?)
    } else {
        None
    };

    skip_ws_and_comments(input)?;
    char_(';').parse_next(input)?;

    let end = input.len();
    let span = Span::new(start - end, start);

    Ok(Statement::VarDecl(VarDecl {
        name,
        type_annotation,
        initializer,
        is_exported,
        span,
    }))
}

/// Парсит объявление функции: [экспорт] функ identifier (params) statements конец
pub fn parse_function_declaration<'a>(input: &mut Input<'a>) -> PResult<'a, Statement> {
    let start = input.len();

    // Опциональное ключевое слово экспорт
    let is_exported = if keyword_export(input).is_ok() {
        skip_ws_and_comments(input)?;
        true
    } else {
        false
    };

    keyword_func(input)?;
    skip_ws_and_comments(input)?;

    let name = parse_identifier(input)?.to_string();
    skip_ws_and_comments(input)?;

    // Параметры
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

    // Тело функции
    let mut body = Vec::new();
    loop {
        skip_ws_and_comments(input)?;

        // Проверяем, не конец ли блока
        let saved = input.checkpoint();
        if keyword_end(input).is_ok() {
            break;
        }
        input.reset(&saved);

        // Парсим оператор
        let stmt = parse_statement(input)?;
        body.push(stmt);
    }

    let end = input.len();
    let span = Span::new(start - end, start);

    Ok(Statement::FuncDecl(FuncDecl {
        name,
        params,
        body,
        annotations: Vec::new(), // TODO: добавить парсинг аннотаций
        is_exported,
        span,
    }))
}

/// Парсит if оператор: если expr statements [иначе statements] конец
pub fn parse_if_statement<'a>(input: &mut Input<'a>) -> PResult<'a, Statement> {
    let start = input.len();

    keyword_if(input)?;
    skip_ws_and_comments(input)?;

    let condition = parse_expression(input)?;
    skip_ws_and_comments(input)?;

    // Тело then
    let mut then_branch = Vec::new();
    loop {
        skip_ws_and_comments(input)?;

        // Проверяем, не иначе или конец ли
        let saved = input.checkpoint();
        if keyword_else(input).is_ok() || keyword_end(input).is_ok() {
            input.reset(&saved);
            break;
        }

        let stmt = parse_statement(input)?;
        then_branch.push(stmt);
    }

    skip_ws_and_comments(input)?;

    // Опциональная ветка else
    let else_branch = if keyword_else(input).is_ok() {
        skip_ws_and_comments(input)?;

        let mut else_stmts = Vec::new();
        loop {
            skip_ws_and_comments(input)?;

            // Проверяем, не конец ли блока
            let saved = input.checkpoint();
            if keyword_end(input).is_ok() {
                input.reset(&saved);
                break;
            }

            let stmt = parse_statement(input)?;
            else_stmts.push(stmt);
        }

        Some(else_stmts)
    } else {
        None
    };

    skip_ws_and_comments(input)?;
    keyword_end(input)?;

    let end = input.len();
    let span = Span::new(start - end, start);

    Ok(Statement::If(IfStatement {
        condition,
        then_branch,
        else_branch,
        span,
    }))
}

/// Парсит while оператор: пока expr statements конец
pub fn parse_while_statement<'a>(input: &mut Input<'a>) -> PResult<'a, Statement> {
    let start = input.len();

    keyword_while(input)?;
    skip_ws_and_comments(input)?;

    let condition = parse_expression(input)?;
    skip_ws_and_comments(input)?;

    // Тело цикла
    let mut body = Vec::new();
    loop {
        skip_ws_and_comments(input)?;

        // Проверяем, не конец ли блока
        let saved = input.checkpoint();
        if keyword_end(input).is_ok() {
            input.reset(&saved);
            break;
        }

        let stmt = parse_statement(input)?;
        body.push(stmt);
    }

    skip_ws_and_comments(input)?;
    keyword_end(input)?;

    let end = input.len();
    let span = Span::new(start - end, start);

    Ok(Statement::While(WhileStatement {
        condition,
        body,
        span,
    }))
}

/// Парсит return оператор: возврат [expr] ;
pub fn parse_return_statement<'a>(input: &mut Input<'a>) -> PResult<'a, Statement> {
    let start = input.len();

    keyword_return(input)?;
    skip_ws_and_comments(input)?;

    // Опциональное возвращаемое значение
    let value = if char_(';').parse_next(input).is_err() {
        Some(parse_expression(input)?)
    } else {
        None
    };

    skip_ws_and_comments(input)?;
    char_(';').parse_next(input)?;

    let end = input.len();
    let span = Span::new(start - end, start);

    Ok(Statement::Return(ReturnStatement {
        value,
        span,
    }))
}

/// Парсит присваивание: (identifier | expr.field) = expression ;
pub fn parse_assignment_statement<'a>(input: &mut Input<'a>) -> PResult<'a, Statement> {
    let start = input.len();

    // Парсим левую часть - может быть identifier или field access
    let target = parse_assignment_target(input)?;
    skip_ws_and_comments(input)?;

    char_('=').parse_next(input)?;
    skip_ws_and_comments(input)?;

    let value = parse_expression(input)?;
    skip_ws_and_comments(input)?;

    char_(';').parse_next(input)?;

    let end = input.len();
    let span = Span::new(start - end, start);

    Ok(Statement::Assignment(AssignmentStatement {
        target,
        value,
        span,
    }))
}

/// Парсит target присваивания (identifier или expr.field)
fn parse_assignment_target<'a>(input: &mut Input<'a>) -> PResult<'a, AssignmentTarget> {
    // Пытаемся спарсить выражение (может быть это.поле, obj.поле и т.д.)
    let expr = parse_expression(input)?;

    // Проверяем, это FieldAccess?
    match expr {
        Expression::Identifier(name, _) => Ok(AssignmentTarget::Identifier(name)),
        Expression::FieldAccess(obj) => {
            Ok(AssignmentTarget::FieldAccess(Box::new(obj.object), obj.field))
        }
        _ => {
            use crate::ponos::parser::error::{PonosParseError, ParseErrorKind};
            Err(winnow::error::ErrMode::Backtrack(PonosParseError::new(
                ParseErrorKind::Custom("Invalid assignment target".to_string()),
                Span::new(0, 1),
            )))
        }
    }
}

/// Парсит присваивание или выражение-оператор
/// Эта функция нужна, потому что оба начинаются с идентификатора
pub fn parse_assignment_or_expression_statement<'a>(input: &mut Input<'a>) -> PResult<'a, Statement> {
    let saved = input.checkpoint();

    // Пытаемся спарсить как присваивание
    if let Ok(assignment) = parse_assignment_statement(input) {
        return Ok(assignment);
    }

    // Если не получилось, откатываемся и парсим как выражение
    input.reset(&saved);
    parse_expression_statement(input)
}

/// Парсит выражение как оператор
pub fn parse_expression_statement<'a>(input: &mut Input<'a>) -> PResult<'a, Statement> {
    let expr = parse_expression(input)?;
    skip_ws_and_comments(input)?;
    char_(';').parse_next(input)?;
    Ok(Statement::Expression(expr))
}

/// Парсит параметр функции
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

// Расширенные конструкции грамматики

/// Парсит импорт: использовать "path" [как псевдоним] ;
pub fn parse_import_statement<'a>(input: &mut Input<'a>) -> PResult<'a, Statement> {
    use crate::ponos::parser::lexer::{keyword_use, keyword_as, parse_string};
    let start = input.len();

    keyword_use(input)?;
    skip_ws_and_comments(input)?;

    let path = parse_string(input)?;
    skip_ws_and_comments(input)?;

    // Опциональный модификатор "как псевдоним"
    let alias = if keyword_as(input).is_ok() {
        skip_ws_and_comments(input)?;
        Some(parse_identifier(input)?.to_string())
    } else {
        None
    };

    skip_ws_and_comments(input)?;
    char_(';').parse_next(input)?;

    let end = input.len();
    let span = Span::new(start - end, start);

    Ok(Statement::Import(ImportStatement {
        path,
        alias,
        span,
    }))
}

/// Вспомогательная функция для парсинга списка идентификаторов через запятую
fn parse_identifier_list<'a>(input: &mut Input<'a>) -> PResult<'a, Vec<String>> {
    let ids: Vec<_> = separated(
        1..,
        |input: &mut Input<'a>| Ok(parse_identifier(input)?.to_string()),
        (skip_ws_and_comments, char_(','), skip_ws_and_comments)
    ).parse_next(input)?;
    Ok(ids)
}

/// Парсит объявление класса: [экспорт] класс identifier [наследует] [реализует] members конец
pub fn parse_class_declaration<'a>(input: &mut Input<'a>) -> PResult<'a, Statement> {
    use crate::ponos::parser::lexer::{keyword_class, keyword_extends, keyword_implements};
    let start = input.len();

    // TODO: parse annotations
    let annotations = Vec::new();

    // Опциональное ключевое слово экспорт
    let is_exported = if keyword_export(input).is_ok() {
        skip_ws_and_comments(input)?;
        true
    } else {
        false
    };

    keyword_class(input)?;
    skip_ws_and_comments(input)?;

    let name = parse_identifier(input)?.to_string();
    skip_ws_and_comments(input)?;

    // Опциональное наследование
    let extends = if keyword_extends(input).is_ok() {
        skip_ws_and_comments(input)?;
        Some(parse_identifier(input)?.to_string())
    } else {
        None
    };

    skip_ws_and_comments(input)?;

    // Опциональная реализация интерфейсов
    let implements = if keyword_implements(input).is_ok() {
        skip_ws_and_comments(input)?;
        parse_identifier_list(input)?
    } else {
        Vec::new()
    };

    skip_ws_and_comments(input)?;

    // Члены класса
    let members = parse_class_members(input)?;

    skip_ws_and_comments(input)?;
    keyword_end(input)?;

    let end = input.len();
    let span = Span::new(start - end, start);

    Ok(Statement::ClassDecl(ClassDecl {
        name,
        extends,
        implements,
        members,
        annotations,
        is_exported,
        span,
    }))
}

fn parse_class_members<'a>(input: &mut Input<'a>) -> PResult<'a, Vec<ClassMember>> {
    let mut members = Vec::new();

    loop {
        skip_ws_and_comments(input)?;

        // Проверяем конец класса
        let saved = input.checkpoint();
        if keyword_end(input).is_ok() {
            input.reset(&saved);
            break;
        }

        // Пытаемся спарсить конструктор
        let saved = input.checkpoint();
        if let Ok(constructor) = parse_constructor_declaration(input) {
            members.push(ClassMember::Constructor(constructor));
            continue;
        }
        input.reset(&saved);

        // Пытаемся спарсить метод
        let saved = input.checkpoint();
        if let Ok(Statement::FuncDecl(func)) = parse_function_declaration(input) {
            members.push(ClassMember::Method(func));
            continue;
        }
        input.reset(&saved);

        // Иначе это поле
        let field_name = parse_identifier(input)?.to_string();
        skip_ws_and_comments(input)?;

        let type_annotation = if char_(':').parse_next(input).is_ok() {
            skip_ws_and_comments(input)?;
            Some(parse_identifier(input)?.to_string())
        } else {
            None
        };

        skip_ws_and_comments(input)?;

        members.push(ClassMember::Field {
            name: field_name,
            type_annotation,
        });

        // Поля могут не иметь точки с запятой в конце, проверяем
        let saved = input.checkpoint();
        if char_(';').parse_next(input).is_err() {
            input.reset(&saved);
        }
    }

    Ok(members)
}

/// Парсит конструктор: конструктор (params) statements конец
pub fn parse_constructor_declaration<'a>(input: &mut Input<'a>) -> PResult<'a, ConstructorDecl> {
    use crate::ponos::parser::lexer::keyword_constructor;
    let start = input.len();

    keyword_constructor(input)?;
    skip_ws_and_comments(input)?;

    // Параметры
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

    // Тело конструктора
    let mut body = Vec::new();
    loop {
        skip_ws_and_comments(input)?;

        let saved = input.checkpoint();
        if keyword_end(input).is_ok() {
            break;
        }
        input.reset(&saved);

        let stmt = parse_statement(input)?;
        body.push(stmt);
    }

    let end = input.len();
    let span = Span::new(start - end, start);

    Ok(ConstructorDecl {
        params,
        body,
        span,
    })
}

/// Парсит объявление интерфейса: [экспорт] интерфейс identifier methods конец
pub fn parse_interface_declaration<'a>(input: &mut Input<'a>) -> PResult<'a, Statement> {
    use crate::ponos::parser::lexer::keyword_interface;
    let start = input.len();

    // Опциональное ключевое слово экспорт
    let is_exported = if keyword_export(input).is_ok() {
        skip_ws_and_comments(input)?;
        true
    } else {
        false
    };

    keyword_interface(input)?;
    skip_ws_and_comments(input)?;

    let name = parse_identifier(input)?.to_string();
    skip_ws_and_comments(input)?;

    // Сигнатуры методов
    let methods = parse_method_signatures(input)?;

    skip_ws_and_comments(input)?;
    keyword_end(input)?;

    let end = input.len();
    let span = Span::new(start - end, start);

    Ok(Statement::InterfaceDecl(InterfaceDecl {
        name,
        methods,
        is_exported,
        span,
    }))
}

fn parse_method_signatures<'a>(input: &mut Input<'a>) -> PResult<'a, Vec<MethodSignature>> {
    let mut methods = Vec::new();

    loop {
        skip_ws_and_comments(input)?;

        let saved = input.checkpoint();
        if keyword_end(input).is_ok() {
            input.reset(&saved);
            break;
        }

        let saved = input.checkpoint();
        match parse_method_signature(input) {
            Ok(sig) => methods.push(sig),
            Err(_) => {
                input.reset(&saved);
                break;
            }
        }
    }

    Ok(methods)
}

fn parse_method_signature<'a>(input: &mut Input<'a>) -> PResult<'a, MethodSignature> {
    let start = input.len();

    keyword_func(input)?;
    skip_ws_and_comments(input)?;

    let name = parse_identifier(input)?.to_string();
    skip_ws_and_comments(input)?;

    // Параметры
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
    char_(';').parse_next(input)?;

    let end = input.len();
    let span = Span::new(start - end, start);

    Ok(MethodSignature {
        name,
        params,
        span,
    })
}

/// Парсит объявление аннотации: [экспорт] аннотация identifier statements конец
pub fn parse_annotation_declaration<'a>(input: &mut Input<'a>) -> PResult<'a, Statement> {
    use crate::ponos::parser::lexer::keyword_annotation;
    let start = input.len();

    // Опциональное ключевое слово экспорт
    let is_exported = if keyword_export(input).is_ok() {
        skip_ws_and_comments(input)?;
        true
    } else {
        false
    };

    keyword_annotation(input)?;
    skip_ws_and_comments(input)?;

    let name = parse_identifier(input)?.to_string();
    skip_ws_and_comments(input)?;

    // Тело аннотации
    let mut body = Vec::new();
    loop {
        skip_ws_and_comments(input)?;

        let saved = input.checkpoint();
        if keyword_end(input).is_ok() {
            break;
        }
        input.reset(&saved);

        let stmt = parse_statement(input)?;
        body.push(stmt);
    }

    let end = input.len();
    let span = Span::new(start - end, start);

    Ok(Statement::AnnotationDecl(AnnotationDecl {
        name,
        body,
        is_exported,
        span,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_var_statement() {
        let mut input = "пер x = 42;";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::VarDecl(decl) => {
                assert_eq!(decl.name, "x");
                assert!(decl.initializer.is_some());
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_parse_var_with_type() {
        let mut input = "пер x: число = 42;";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::VarDecl(decl) => {
                assert_eq!(decl.name, "x");
                assert_eq!(decl.type_annotation, Some("число".to_string()));
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_parse_function() {
        let mut input = "функ foo(x, y) возврат x + y; конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::FuncDecl(func) => {
                assert_eq!(func.name, "foo");
                assert_eq!(func.params.len(), 2);
                assert_eq!(func.body.len(), 1);
            }
            _ => panic!("Expected FuncDecl"),
        }
    }

    #[test]
    fn test_parse_function_with_typed_params() {
        let mut input = "функ типизированная(парам1: число, парам2: строка) возврат парам1; конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::FuncDecl(func) => {
                assert_eq!(func.name, "типизированная");
                assert_eq!(func.params.len(), 2);

                // Проверяем первый параметр
                assert_eq!(func.params[0].name, "парам1");
                assert_eq!(func.params[0].type_annotation, Some("число".to_string()));

                // Проверяем второй параметр
                assert_eq!(func.params[1].name, "парам2");
                assert_eq!(func.params[1].type_annotation, Some("строка".to_string()));
            }
            _ => panic!("Expected FuncDecl"),
        }
    }

    #[test]
    fn test_parse_function_with_mixed_params() {
        let mut input = "функ смешанная(без_типа, с_типом: число) конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::FuncDecl(func) => {
                assert_eq!(func.name, "смешанная");
                assert_eq!(func.params.len(), 2);

                // Проверяем параметр без типа
                assert_eq!(func.params[0].name, "без_типа");
                assert_eq!(func.params[0].type_annotation, None);

                // Проверяем параметр с типом
                assert_eq!(func.params[1].name, "с_типом");
                assert_eq!(func.params[1].type_annotation, Some("число".to_string()));
            }
            _ => panic!("Expected FuncDecl"),
        }
    }

    #[test]
    fn test_parse_if() {
        let mut input = "если x > 0 возврат x; конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::If(if_stmt) => {
                assert_eq!(if_stmt.then_branch.len(), 1);
                assert!(if_stmt.else_branch.is_none());
            }
            _ => panic!("Expected If"),
        }
    }

    #[test]
    fn test_parse_if_else() {
        let mut input = "если x > 0 возврат x; иначе возврат 0; конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::If(if_stmt) => {
                assert_eq!(if_stmt.then_branch.len(), 1);
                assert!(if_stmt.else_branch.is_some());
                assert_eq!(if_stmt.else_branch.unwrap().len(), 1);
            }
            _ => panic!("Expected If"),
        }
    }

    #[test]
    fn test_parse_while() {
        let mut input = "пока x > 0 вызов(); конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::While(while_stmt) => {
                assert_eq!(while_stmt.body.len(), 1);
            }
            _ => panic!("Expected While"),
        }
    }

    #[test]
    fn test_parse_return() {
        let mut input = "возврат 42;";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::Return(ret) => {
                assert!(ret.value.is_some());
            }
            _ => panic!("Expected Return"),
        }
    }

    #[test]
    fn test_parse_expression_statement() {
        let mut input = "foo();";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::Expression(_) => {}
            _ => panic!("Expected Expression statement"),
        }
    }

    #[test]
    fn test_parse_assignment() {
        let mut input = "x = 42;";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::Assignment(assign) => {
                match &assign.target {
                    AssignmentTarget::Identifier(name) => assert_eq!(name, "x"),
                    _ => panic!("Expected identifier target"),
                }
                match assign.value {
                    Expression::Number(n, _) => assert_eq!(n, 42.0),
                    _ => panic!("Expected number"),
                }
            }
            _ => panic!("Expected Assignment"),
        }
    }

    #[test]
    fn test_parse_assignment_with_expression() {
        let mut input = "результат = x + y;";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::Assignment(assign) => {
                match &assign.target {
                    AssignmentTarget::Identifier(name) => assert_eq!(name, "результат"),
                    _ => panic!("Expected identifier target"),
                }
            }
            _ => panic!("Expected Assignment"),
        }
    }

    #[test]
    fn test_parse_class_simple() {
        let mut input = "класс Точка конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::ClassDecl(class) => {
                assert_eq!(class.name, "Точка");
                assert!(class.extends.is_none());
                assert!(class.implements.is_empty());
                assert!(class.members.is_empty());
            }
            _ => panic!("Expected ClassDecl"),
        }
    }

    #[test]
    fn test_parse_class_with_extends() {
        let mut input = "класс Подкласс наследует Родитель конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::ClassDecl(class) => {
                assert_eq!(class.name, "Подкласс");
                assert_eq!(class.extends, Some("Родитель".to_string()));
            }
            _ => panic!("Expected ClassDecl"),
        }
    }

    #[test]
    fn test_parse_class_with_implements() {
        let mut input = "класс Реализатор реализует Интерфейс1, Интерфейс2 конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::ClassDecl(class) => {
                assert_eq!(class.name, "Реализатор");
                assert_eq!(class.implements.len(), 2);
                assert_eq!(class.implements[0], "Интерфейс1");
                assert_eq!(class.implements[1], "Интерфейс2");
            }
            _ => panic!("Expected ClassDecl"),
        }
    }

    #[test]
    fn test_parse_class_with_field() {
        let mut input = "класс Человек имя: строка конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::ClassDecl(class) => {
                assert_eq!(class.name, "Человек");
                assert_eq!(class.members.len(), 1);
                match &class.members[0] {
                    ClassMember::Field { name, type_annotation } => {
                        assert_eq!(name, "имя");
                        assert_eq!(type_annotation, &Some("строка".to_string()));
                    }
                    _ => panic!("Expected Field"),
                }
            }
            _ => panic!("Expected ClassDecl"),
        }
    }

    #[test]
    fn test_parse_class_with_constructor() {
        let mut input = "класс Точка конструктор(x: число, y: число) конец конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::ClassDecl(class) => {
                assert_eq!(class.name, "Точка");
                assert_eq!(class.members.len(), 1);
                match &class.members[0] {
                    ClassMember::Constructor(ctor) => {
                        assert_eq!(ctor.params.len(), 2);
                        assert_eq!(ctor.params[0].name, "x");
                        assert_eq!(ctor.params[1].name, "y");
                    }
                    _ => panic!("Expected Constructor"),
                }
            }
            _ => panic!("Expected ClassDecl"),
        }
    }

    #[test]
    fn test_parse_class_with_method() {
        let mut input = "класс Точка функ печать() конец конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::ClassDecl(class) => {
                assert_eq!(class.name, "Точка");
                assert_eq!(class.members.len(), 1);
                match &class.members[0] {
                    ClassMember::Method(method) => {
                        assert_eq!(method.name, "печать");
                    }
                    _ => panic!("Expected Method"),
                }
            }
            _ => panic!("Expected ClassDecl"),
        }
    }

    #[test]
    fn test_parse_interface() {
        let mut input = "интерфейс Печатаемый функ печать(); конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::InterfaceDecl(interface) => {
                assert_eq!(interface.name, "Печатаемый");
                assert_eq!(interface.methods.len(), 1);
                assert_eq!(interface.methods[0].name, "печать");
            }
            _ => panic!("Expected InterfaceDecl"),
        }
    }

    #[test]
    fn test_parse_interface_with_params() {
        let mut input = "интерфейс Сравнимый функ равно(другой: Сравнимый); конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::InterfaceDecl(interface) => {
                assert_eq!(interface.name, "Сравнимый");
                assert_eq!(interface.methods.len(), 1);
                assert_eq!(interface.methods[0].name, "равно");
                assert_eq!(interface.methods[0].params.len(), 1);
            }
            _ => panic!("Expected InterfaceDecl"),
        }
    }

    #[test]
    fn test_parse_annotation() {
        let mut input = "аннотация Тест конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::AnnotationDecl(annotation) => {
                assert_eq!(annotation.name, "Тест");
                assert!(annotation.body.is_empty());
            }
            _ => panic!("Expected AnnotationDecl"),
        }
    }

    #[test]
    fn test_parse_import_simple() {
        let mut input = r#"использовать "путь/к/модулю";"#;
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::Import(import) => {
                assert_eq!(import.path, "путь/к/модулю");
                assert!(import.alias.is_none());
            }
            _ => panic!("Expected Import"),
        }
    }

    #[test]
    fn test_parse_import_with_as() {
        let mut input = r#"использовать "модуль" как М;"#;
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::Import(import) => {
                assert_eq!(import.path, "модуль");
                assert_eq!(import.alias, Some("М".to_string()));
            }
            _ => panic!("Expected Import"),
        }
    }

    #[test]
    fn test_parse_field_access_assignment() {
        let mut input = "obj.поле = 42;";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::Assignment(assign) => {
                match &assign.target {
                    AssignmentTarget::FieldAccess(_, field) => {
                        assert_eq!(field, "поле");
                    }
                    _ => panic!("Expected field access target"),
                }
            }
            _ => panic!("Expected Assignment"),
        }
    }

    // Тесты для экспорта

    #[test]
    fn test_parse_exported_var() {
        let mut input = "экспорт пер x = 42;";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::VarDecl(decl) => {
                assert_eq!(decl.name, "x");
                assert_eq!(decl.is_exported, true);
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_parse_non_exported_var() {
        let mut input = "пер x = 42;";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::VarDecl(decl) => {
                assert_eq!(decl.name, "x");
                assert_eq!(decl.is_exported, false);
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_parse_exported_function() {
        let mut input = "экспорт функ foo() конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::FuncDecl(func) => {
                assert_eq!(func.name, "foo");
                assert_eq!(func.is_exported, true);
            }
            _ => panic!("Expected FuncDecl"),
        }
    }

    #[test]
    fn test_parse_non_exported_function() {
        let mut input = "функ foo() конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::FuncDecl(func) => {
                assert_eq!(func.name, "foo");
                assert_eq!(func.is_exported, false);
            }
            _ => panic!("Expected FuncDecl"),
        }
    }

    #[test]
    fn test_parse_exported_class() {
        let mut input = "экспорт класс Точка конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::ClassDecl(class) => {
                assert_eq!(class.name, "Точка");
                assert_eq!(class.is_exported, true);
            }
            _ => panic!("Expected ClassDecl"),
        }
    }

    #[test]
    fn test_parse_non_exported_class() {
        let mut input = "класс Точка конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::ClassDecl(class) => {
                assert_eq!(class.name, "Точка");
                assert_eq!(class.is_exported, false);
            }
            _ => panic!("Expected ClassDecl"),
        }
    }

    #[test]
    fn test_parse_exported_interface() {
        let mut input = "экспорт интерфейс Печатаемый конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::InterfaceDecl(interface) => {
                assert_eq!(interface.name, "Печатаемый");
                assert_eq!(interface.is_exported, true);
            }
            _ => panic!("Expected InterfaceDecl"),
        }
    }

    #[test]
    fn test_parse_non_exported_interface() {
        let mut input = "интерфейс Печатаемый конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::InterfaceDecl(interface) => {
                assert_eq!(interface.name, "Печатаемый");
                assert_eq!(interface.is_exported, false);
            }
            _ => panic!("Expected InterfaceDecl"),
        }
    }

    #[test]
    fn test_parse_exported_annotation() {
        let mut input = "экспорт аннотация Тест конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::AnnotationDecl(annotation) => {
                assert_eq!(annotation.name, "Тест");
                assert_eq!(annotation.is_exported, true);
            }
            _ => panic!("Expected AnnotationDecl"),
        }
    }

    #[test]
    fn test_parse_non_exported_annotation() {
        let mut input = "аннотация Тест конец";
        let stmt = parse_statement(&mut input).unwrap();
        match stmt {
            Statement::AnnotationDecl(annotation) => {
                assert_eq!(annotation.name, "Тест");
                assert_eq!(annotation.is_exported, false);
            }
            _ => panic!("Expected AnnotationDecl"),
        }
    }
}
