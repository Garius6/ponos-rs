use crate::ponos::ast::{Expression, Program, Statement};
use crate::ponos::symbol_table::{SymbolKind, SymbolTable};

/// Разрешитель имен - преобразует FieldAccess в ModuleAccess где необходимо
///
/// Проходит по AST и преобразует выражения вида `модуль.символ` из `FieldAccess`
/// в `ModuleAccess`, если `модуль` является зарегистрированным пространством имен (Symbol::Module)
/// и символ экспортирован из этого модуля.
pub struct NameResolver;

impl NameResolver {
    /// Создать новый разрешитель имен
    pub fn new() -> Self {
        NameResolver
    }

    /// Разрешить имена в AST программы
    ///
    /// # Параметры
    /// - `ast`: AST программы для обработки
    /// - `symbol_table`: Таблица символов с зарегистрированными модулями
    ///
    /// # Возвращает
    /// `Ok(())` при успехе, `Err(String)` при ошибке
    pub fn resolve(&mut self, ast: &mut Program, symbol_table: &SymbolTable) -> Result<(), String> {
        // Обрабатываем все statements
        for statement in &mut ast.statements {
            self.resolve_statement(statement, symbol_table)?;
        }

        Ok(())
    }

    /// Разрешить имена в statement
    fn resolve_statement(
        &mut self,
        stmt: &mut Statement,
        symbol_table: &SymbolTable,
    ) -> Result<(), String> {
        use crate::ponos::ast::AssignmentTarget;

        match stmt {
            Statement::VarDecl(var_decl) => {
                if let Some(init) = &mut var_decl.initializer {
                    self.resolve_expression(init, symbol_table)?;
                }
            }
            Statement::FuncDecl(func_decl) => {
                for stmt in &mut func_decl.body {
                    self.resolve_statement(stmt, symbol_table)?;
                }
            }
            Statement::Assignment(assign) => {
                // Обрабатываем значение
                self.resolve_expression(&mut assign.value, symbol_table)?;

                // Обрабатываем target (может быть FieldAccess)
                if let AssignmentTarget::FieldAccess(obj, _) = &mut assign.target {
                    self.resolve_expression(obj, symbol_table)?;
                }
            }
            Statement::If(if_stmt) => {
                self.resolve_expression(&mut if_stmt.condition, symbol_table)?;
                for stmt in &mut if_stmt.then_branch {
                    self.resolve_statement(stmt, symbol_table)?;
                }
                if let Some(else_branch) = &mut if_stmt.else_branch {
                    for stmt in else_branch {
                        self.resolve_statement(stmt, symbol_table)?;
                    }
                }
            }
            Statement::While(while_stmt) => {
                self.resolve_expression(&mut while_stmt.condition, symbol_table)?;
                for stmt in &mut while_stmt.body {
                    self.resolve_statement(stmt, symbol_table)?;
                }
            }
            Statement::ForEach(foreach_stmt) => {
                self.resolve_expression(&mut foreach_stmt.iterable, symbol_table)?;
                for stmt in &mut foreach_stmt.body {
                    self.resolve_statement(stmt, symbol_table)?;
                }
            }
            Statement::Return(ret_stmt) => {
                if let Some(value) = &mut ret_stmt.value {
                    self.resolve_expression(value, symbol_table)?;
                }
            }
            Statement::Expression(expr) => {
                self.resolve_expression(expr, symbol_table)?;
            }
            Statement::ModuleBlock(module_block) => {
                for stmt in &mut module_block.statements {
                    self.resolve_statement(stmt, symbol_table)?;
                }
            }
            // Остальные statements не содержат выражений
            _ => {}
        }
        Ok(())
    }

    /// Разрешить имена в выражении
    ///
    /// Ключевая функция: преобразует `FieldAccess` в `ModuleAccess` если нужно
    fn resolve_expression(
        &mut self,
        expr: &mut Expression,
        symbol_table: &SymbolTable,
    ) -> Result<(), String> {
        use crate::ponos::ast::ModuleAccessExpr;

        match expr {
            Expression::Binary(binary) => {
                self.resolve_expression(&mut binary.left, symbol_table)?;
                self.resolve_expression(&mut binary.right, symbol_table)?;
            }
            Expression::Unary(unary) => {
                self.resolve_expression(&mut unary.operand, symbol_table)?;
            }
            Expression::Call(call) => {
                self.resolve_expression(&mut call.callee, symbol_table)?;
                for arg in &mut call.arguments {
                    self.resolve_expression(arg, symbol_table)?;
                }
            }
            Expression::FieldAccess(field_access) => {
                // КРИТИЧЕСКИЙ МОМЕНТ: проверяем, это доступ к модулю или к полю объекта

                // Сначала рекурсивно обрабатываем объект
                self.resolve_expression(&mut field_access.object, symbol_table)?;

                // Проверяем, является ли объект идентификатором из пространства имен
                if let Expression::Identifier(name, _span) = &field_access.object {
                    // Ищем символ в SymbolTable
                    if let Some(module_symbol) = symbol_table.lookup(name) {
                        // Проверяем, это модуль?
                        if module_symbol.kind == SymbolKind::Module {
                            // Это зарегистрированное пространство имен!
                            let module_scope_id = module_symbol
                                .module_scope_id
                                .ok_or_else(|| format!("Модуль '{}' не имеет scope_id", name))?;

                            // Проверяем, что символ экспортирован из модуля
                            let symbol_name = &field_access.field;
                            match symbol_table.lookup_in_scope(module_scope_id, symbol_name) {
                                Some(symbol) if symbol.is_exported => {
                                    // Символ существует и экспортирован - преобразуем в ModuleAccess
                                    let module_access = ModuleAccessExpr {
                                        namespace: name.clone(),
                                        symbol: field_access.field.clone(),
                                        span: field_access.span,
                                    };
                                    *expr = Expression::ModuleAccess(Box::new(module_access));
                                }
                                Some(_) => {
                                    // Символ существует, но не экспортирован
                                    return Err(format!(
                                        "Символ '{}' не экспортирован из модуля '{}'",
                                        symbol_name, name
                                    ));
                                }
                                None => {
                                    // Символ не найден в модуле
                                    return Err(format!(
                                        "Символ '{}' не найден в модуле '{}'",
                                        symbol_name, name
                                    ));
                                }
                            }
                        }
                        // Если это не модуль, оставляем как FieldAccess (обычный доступ к полю объекта)
                    }
                    // Если символ не найден, тоже оставляем как FieldAccess (может быть локальная переменная)
                }
            }
            Expression::Lambda(lambda) => {
                for stmt in &mut lambda.body {
                    self.resolve_statement(stmt, symbol_table)?;
                }
            }
            // Остальные выражения не содержат вложенных выражений или уже разрешены
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ponos::ast::{
        BinaryExpr, CallExpr, Expression, FieldAccessExpr, ModuleAccessExpr, Program, Statement,
        VarDecl,
    };
    use crate::ponos::span::Span;
    use crate::ponos::symbol_table::{ScopeId, Symbol, SymbolKind, SymbolTable};

    /// Вспомогательная функция для создания SymbolTable с модулем и экспортом
    /// Регистрирует namespace как Symbol::Module в главном scope
    fn create_module_with_export(module_name: &str, symbol_name: &str) -> SymbolTable {
        let mut symbol_table = SymbolTable::new();

        // Создаём scope для модуля
        let module_scope_id = symbol_table.push_scope();

        // Регистрируем экспортированный символ в scope модуля
        let symbol = Symbol::new(
            symbol_name.to_string(),
            SymbolKind::Variable,
            true,
            Span::default(),
        );
        symbol_table
            .define_in_scope(module_scope_id, symbol)
            .unwrap();

        // Возвращаемся в главный scope и регистрируем namespace как Symbol::Module
        symbol_table.pop_scope();
        let module_symbol =
            Symbol::new_module(module_name.to_string(), module_scope_id, Span::default());
        symbol_table.define(module_symbol).unwrap();

        symbol_table
    }

    #[test]
    fn test_name_resolver_creation() {
        let _resolver = NameResolver::new();
        // NameResolver теперь не имеет состояния, просто проверяем что создаётся
    }

    #[test]
    fn test_resolve_field_access_to_module_access() {
        // Создаем AST: пер x = модуль.символ;
        let mut ast = Program {
            statements: vec![Statement::VarDecl(VarDecl {
                name: "x".to_string(),
                type_annotation: None,
                initializer: Some(Expression::FieldAccess(Box::new(FieldAccessExpr {
                    object: Expression::Identifier("модуль".to_string(), Span::default()),
                    field: "символ".to_string(),
                    span: Span::default(),
                }))),
                is_exported: false,
                span: Span::default(),
            })],
        };

        let symbol_table = create_module_with_export("модуль", "символ");
        let mut resolver = NameResolver::new();
        resolver.resolve(&mut ast, &symbol_table).unwrap();

        // Проверяем, что FieldAccess был преобразован в ModuleAccess
        match &ast.statements[0] {
            Statement::VarDecl(var_decl) => match &var_decl.initializer {
                Some(Expression::ModuleAccess(module_access)) => {
                    assert_eq!(module_access.namespace, "модуль");
                    assert_eq!(module_access.symbol, "символ");
                }
                _ => panic!("Expected ModuleAccess"),
            },
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_does_not_convert_regular_field_access() {
        // Создаем AST: пер x = объект.поле;
        let mut ast = Program {
            statements: vec![Statement::VarDecl(VarDecl {
                name: "x".to_string(),
                type_annotation: None,
                initializer: Some(Expression::FieldAccess(Box::new(FieldAccessExpr {
                    object: Expression::Identifier("объект".to_string(), Span::default()),
                    field: "поле".to_string(),
                    span: Span::default(),
                }))),
                is_exported: false,
                span: Span::default(),
            })],
        };

        // Создаём модуль, но НЕ регистрируем "объект" как пространство имен
        let symbol_table = create_module_with_export("модуль", "символ");
        let mut resolver = NameResolver::new();
        resolver.resolve(&mut ast, &symbol_table).unwrap();

        // Проверяем, что FieldAccess остался без изменений
        match &ast.statements[0] {
            Statement::VarDecl(var_decl) => match &var_decl.initializer {
                Some(Expression::FieldAccess(field_access)) => {
                    match &field_access.object {
                        Expression::Identifier(name, _) => {
                            assert_eq!(name, "объект");
                        }
                        _ => panic!("Expected Identifier"),
                    }
                    assert_eq!(field_access.field, "поле");
                }
                _ => panic!("Expected FieldAccess, not ModuleAccess"),
            },
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_resolve_in_binary_expression() {
        // Создаем AST: пер x = модуль.символ + 5;
        let mut ast = Program {
            statements: vec![Statement::VarDecl(VarDecl {
                name: "x".to_string(),
                type_annotation: None,
                initializer: Some(Expression::Binary(Box::new(BinaryExpr {
                    left: Expression::FieldAccess(Box::new(FieldAccessExpr {
                        object: Expression::Identifier("модуль".to_string(), Span::default()),
                        field: "символ".to_string(),
                        span: Span::default(),
                    })),
                    operator: crate::ponos::ast::BinaryOperator::Add,
                    right: Expression::Number(5.0, Span::default()),
                    span: Span::default(),
                }))),
                is_exported: false,
                span: Span::default(),
            })],
        };

        let symbol_table = create_module_with_export("модуль", "символ");
        let mut resolver = NameResolver::new();
        resolver.resolve(&mut ast, &symbol_table).unwrap();

        // Проверяем, что FieldAccess в левой части был преобразован
        match &ast.statements[0] {
            Statement::VarDecl(var_decl) => match &var_decl.initializer {
                Some(Expression::Binary(binary)) => match &binary.left {
                    Expression::ModuleAccess(module_access) => {
                        assert_eq!(module_access.namespace, "модуль");
                        assert_eq!(module_access.symbol, "символ");
                    }
                    _ => panic!("Expected ModuleAccess in left operand"),
                },
                _ => panic!("Expected Binary expression"),
            },
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_resolve_in_function_call() {
        // Создаем AST: модуль.функция();
        let mut ast = Program {
            statements: vec![Statement::Expression(Expression::Call(Box::new(
                CallExpr {
                    callee: Expression::FieldAccess(Box::new(FieldAccessExpr {
                        object: Expression::Identifier("модуль".to_string(), Span::default()),
                        field: "функция".to_string(),
                        span: Span::default(),
                    })),
                    arguments: vec![],
                    span: Span::default(),
                },
            )))],
        };

        let symbol_table = create_module_with_export("модуль", "функция");
        let mut resolver = NameResolver::new();
        resolver.resolve(&mut ast, &symbol_table).unwrap();

        // Проверяем, что callee был преобразован
        match &ast.statements[0] {
            Statement::Expression(Expression::Call(call)) => match &call.callee {
                Expression::ModuleAccess(module_access) => {
                    assert_eq!(module_access.namespace, "модуль");
                    assert_eq!(module_access.symbol, "функция");
                }
                _ => panic!("Expected ModuleAccess in callee"),
            },
            _ => panic!("Expected Call expression"),
        }
    }

    // Тест test_is_namespace удалён, так как метод is_namespace больше не существует

    #[test]
    fn test_resolve_multiple_namespaces() {
        // Создаем AST: пер x = мод1.символ + мод2.символ;
        let mut ast = Program {
            statements: vec![Statement::VarDecl(VarDecl {
                name: "x".to_string(),
                type_annotation: None,
                initializer: Some(Expression::Binary(Box::new(BinaryExpr {
                    left: Expression::FieldAccess(Box::new(FieldAccessExpr {
                        object: Expression::Identifier("мод1".to_string(), Span::default()),
                        field: "символ".to_string(),
                        span: Span::default(),
                    })),
                    operator: crate::ponos::ast::BinaryOperator::Add,
                    right: Expression::FieldAccess(Box::new(FieldAccessExpr {
                        object: Expression::Identifier("мод2".to_string(), Span::default()),
                        field: "символ".to_string(),
                        span: Span::default(),
                    })),
                    span: Span::default(),
                }))),
                is_exported: false,
                span: Span::default(),
            })],
        };

        // Создаём два модуля с экспортированными символами
        let mut symbol_table = SymbolTable::new();

        // Создаём первый модуль
        let scope_id1 = symbol_table.push_scope();
        symbol_table
            .define_in_scope(
                scope_id1,
                Symbol::new(
                    "символ".to_string(),
                    SymbolKind::Variable,
                    true,
                    Span::default(),
                ),
            )
            .unwrap();
        symbol_table.pop_scope();
        symbol_table
            .define(Symbol::new_module(
                "мод1".to_string(),
                scope_id1,
                Span::default(),
            ))
            .unwrap();

        // Создаём второй модуль
        let scope_id2 = symbol_table.push_scope();
        symbol_table
            .define_in_scope(
                scope_id2,
                Symbol::new(
                    "символ".to_string(),
                    SymbolKind::Variable,
                    true,
                    Span::default(),
                ),
            )
            .unwrap();
        symbol_table.pop_scope();
        symbol_table
            .define(Symbol::new_module(
                "мод2".to_string(),
                scope_id2,
                Span::default(),
            ))
            .unwrap();

        let mut resolver = NameResolver::new();
        resolver.resolve(&mut ast, &symbol_table).unwrap();

        // Проверяем, что оба FieldAccess были преобразованы
        match &ast.statements[0] {
            Statement::VarDecl(var_decl) => {
                match &var_decl.initializer {
                    Some(Expression::Binary(binary)) => {
                        // Левая часть
                        match &binary.left {
                            Expression::ModuleAccess(module_access) => {
                                assert_eq!(module_access.namespace, "мод1");
                            }
                            _ => panic!("Expected ModuleAccess in left"),
                        }
                        // Правая часть
                        match &binary.right {
                            Expression::ModuleAccess(module_access) => {
                                assert_eq!(module_access.namespace, "мод2");
                            }
                            _ => panic!("Expected ModuleAccess in right"),
                        }
                    }
                    _ => panic!("Expected Binary"),
                }
            }
            _ => panic!("Expected VarDecl"),
        }
    }
}
