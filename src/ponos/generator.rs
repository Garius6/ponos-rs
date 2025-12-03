pub struct Generator {}

use crate::ponos::ast::UnaryOperator;

use super::ast::{
    AstNode, AssignmentTarget, Expression, ImportStatement, Statement,
};
use super::opcode::OpCode;
use super::value::Value;
use std::collections::HashMap;

pub struct GenContext {
    pub constants: Vec<Value>,
    pub opcodes: Vec<OpCode>,
    pub current_namespace: Option<String>, // Префикс для манглинга имен (было current_module)
    pub in_function: bool,
    local_slots: HashMap<String, usize>,
    next_local_slot: usize,
}

impl Generator {
    pub fn new() -> Self {
        Generator {}
    }

    pub fn generate(&mut self, node: AstNode) -> GenContext {
        let mut context = self.make_context(false);
        match node {
            AstNode::Program(program) => {
                for stmt in program.statements {
                    self.emit_statement(stmt, &mut context);
                }
            }
            _ => panic!("Неверно сгенерировано ast"),
        }
        context
    }

    pub fn generate_function_body(&mut self, statements: Vec<Statement>) -> GenContext {
        let mut ctx = self.make_context(true);
        for stmt in statements {
            self.emit_statement(stmt, &mut ctx);
        }
        ctx
    }

    fn make_context(&self, in_function: bool) -> GenContext {
        GenContext {
            constants: Vec::new(),
            opcodes: Vec::new(),
            current_namespace: None,
            in_function,
            local_slots: HashMap::new(),
            next_local_slot: 0,
        }
    }

    fn emit_statement(&mut self, stmt: Statement, ctx: &mut GenContext) {
        match stmt {
            Statement::ModuleDecl(_module) => {
                // ModuleDecl больше не генерирует опкоды - пространство имен устанавливается в ModuleBlock
            }
            Statement::VarDecl(var_decl) => {
                if ctx.in_function {
                    if var_decl.is_exported {
                        panic!("Нельзя экспортировать переменную внутри функции");
                    }
                    self.emit_local_var_decl(var_decl.name, var_decl.initializer, ctx);
                } else {
                    // Генерируем значение
                    if let Some(init) = var_decl.initializer {
                        self.emit_expression(init, ctx);
                    } else {
                        let idx = self.intern_constant(Value::Nil, ctx);
                        ctx.opcodes.push(OpCode::Constant(idx));
                    }

                    // Применяем манглинг имени если есть пространство имен
                    let mangled_name = self.mangle_name(&var_decl.name, ctx);
                    let name_idx = self.intern_string(&mangled_name, ctx);
                    ctx.opcodes.push(OpCode::DefineGlobal(name_idx));

                    // ExportSymbol больше не нужен - экспорты обрабатываются на этапе разрешения имен
                }
            }
            Statement::Import(_import_stmt) => {
                // Import больше не генерирует опкоды - всё обработано на этапе разрешения имен
            }
            Statement::Assignment(assign) => {
                self.emit_expression(assign.value, ctx);
                match assign.target {
                    AssignmentTarget::Identifier(name) => {
                        if ctx.in_function {
                            if let Some(slot) = ctx.local_slots.get(&name) {
                                ctx.opcodes.push(OpCode::SetLocal(*slot));
                            } else {
                                let mangled_name = self.mangle_name(&name, ctx);
                                let name_idx = self.intern_string(&mangled_name, ctx);
                                ctx.opcodes.push(OpCode::SetGlobal(name_idx));
                            }
                        } else {
                            let mangled_name = self.mangle_name(&name, ctx);
                            let name_idx = self.intern_string(&mangled_name, ctx);
                            ctx.opcodes.push(OpCode::SetGlobal(name_idx));
                        }
                    }
                    AssignmentTarget::FieldAccess(_, _) => {
                        unimplemented!("Присваивание в поле пока не поддерживается")
                    }
                }
            }
            Statement::Expression(e) => self.emit_expression(e, ctx),
            Statement::ModuleBlock(module_block) => {
                // Сохраняем текущее пространство имен
                let previous_namespace = ctx.current_namespace.clone();

                // Устанавливаем пространство имен для statements модуля
                ctx.current_namespace = Some(module_block.namespace.clone());

                // Генерируем код для всех statements модуля
                for stmt in module_block.statements {
                    self.emit_statement(stmt, ctx);
                }

                // Восстанавливаем предыдущее пространство имен
                ctx.current_namespace = previous_namespace;
            }
            _ => unimplemented!(),
        }
    }

    fn emit_expression(&mut self, e: Expression, ctx: &mut GenContext) {
        match e {
            Expression::Number(n, _) => {
                let idx = self.intern_constant(Value::Number(n), ctx);
                ctx.opcodes.push(OpCode::Constant(idx))
            }
            Expression::String(s, _) => {
                let idx = self.intern_constant(Value::String(s), ctx);
                ctx.opcodes.push(OpCode::Constant(idx))
            }
            Expression::Boolean(b, _) => {
                let idx = self.intern_constant(Value::Boolean(b), ctx);
                ctx.opcodes.push(OpCode::Constant(idx))
            }
            Expression::Identifier(name, _) => {
                if ctx.in_function {
                    if let Some(slot) = ctx.local_slots.get(&name) {
                        ctx.opcodes.push(OpCode::GetLocal(*slot));
                    } else {
                        let mangled_name = self.mangle_name(&name, ctx);
                        let name_idx = self.intern_string(&mangled_name, ctx);
                        ctx.opcodes.push(OpCode::GetGlobal(name_idx));
                    }
                } else {
                    let mangled_name = self.mangle_name(&name, ctx);
                    let name_idx = self.intern_string(&mangled_name, ctx);
                    ctx.opcodes.push(OpCode::GetGlobal(name_idx));
                }
            }
            Expression::Binary(binary_expr) => {
                self.emit_expression(binary_expr.left, ctx);
                self.emit_expression(binary_expr.right, ctx);
                let mut ops = match binary_expr.operator {
                    crate::ponos::ast::BinaryOperator::Add => vec![OpCode::Add],
                    crate::ponos::ast::BinaryOperator::Subtract => vec![OpCode::Sub],
                    crate::ponos::ast::BinaryOperator::Multiply => vec![OpCode::Mul],
                    crate::ponos::ast::BinaryOperator::Divide => vec![OpCode::Div],
                    crate::ponos::ast::BinaryOperator::Equal => vec![OpCode::Eql],
                    crate::ponos::ast::BinaryOperator::NotEqual => vec![OpCode::Eql, OpCode::Not],
                    crate::ponos::ast::BinaryOperator::Less => vec![OpCode::Less],
                    crate::ponos::ast::BinaryOperator::LessEqual => {
                        vec![OpCode::Greater, OpCode::Not]
                    }
                    crate::ponos::ast::BinaryOperator::Greater => vec![OpCode::Greater],
                    crate::ponos::ast::BinaryOperator::GreaterEqual => {
                        vec![OpCode::Less, OpCode::Not]
                    }
                    crate::ponos::ast::BinaryOperator::And => todo!(),
                    crate::ponos::ast::BinaryOperator::Or => todo!(),
                };
                ctx.opcodes.append(&mut ops);
            }
            Expression::Unary(unary_expr) => {
                self.emit_expression(unary_expr.operand, ctx);
                let op = match unary_expr.operator {
                    UnaryOperator::Negate => OpCode::Negate,
                    UnaryOperator::Not => OpCode::Not,
                };
                ctx.opcodes.push(op);
            }
            Expression::Call(call_expr) => todo!(),
            Expression::FieldAccess(field_access_expr) => todo!(),
            Expression::ModuleAccess(module_access) => {
                // Генерируем загрузку символа из модуля с манглингом имен
                let mangled_name = format!("{}::{}", module_access.namespace, module_access.symbol);
                let name_idx = self.intern_string(&mangled_name, ctx);
                ctx.opcodes.push(OpCode::GetGlobal(name_idx));
            }
            Expression::Lambda(lambda_expr) => todo!(),
            Expression::This(span) => todo!(),
            Expression::Super(_, span) => todo!(),
        }
    }

    /// Применить манглинг имени переменной с учетом текущего пространства имен
    fn mangle_name(&self, name: &str, ctx: &GenContext) -> String {
        if let Some(namespace) = &ctx.current_namespace {
            format!("{}::{}", namespace, name)
        } else {
            name.to_string()
        }
    }

    fn emit_local_var_decl(
        &mut self,
        name: String,
        initializer: Option<Expression>,
        ctx: &mut GenContext,
    ) {
        if let Some(init) = initializer {
            self.emit_expression(init, ctx);
        } else {
            let idx = self.intern_constant(Value::Nil, ctx);
            ctx.opcodes.push(OpCode::Constant(idx));
        }

        let slot = if let Some(slot) = ctx.local_slots.get(&name) {
            *slot
        } else {
            let slot = ctx.next_local_slot;
            ctx.next_local_slot += 1;
            ctx.local_slots.insert(name, slot);
            slot
        };

        ctx.opcodes.push(OpCode::DefineLocal(slot));
    }


    fn intern_string(&mut self, value: &str, ctx: &mut GenContext) -> usize {
        self.intern_constant(Value::String(value.to_string()), ctx)
    }

    fn intern_constant(&mut self, value: Value, ctx: &mut GenContext) -> usize {
        if let Some(idx) = ctx.constants.iter().position(|v| v == &value) {
            idx
        } else {
            ctx.constants.push(value);
            ctx.constants.len() - 1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ponos::ast::{Program, VarDecl};
    use crate::ponos::span::Span;

    fn number_expr(n: f64) -> Expression {
        Expression::Number(n, Span::default())
    }

    #[test]
    fn generates_default_module_and_global_define() {
        let program = Program {
            statements: vec![Statement::VarDecl(VarDecl {
                name: "a".to_string(),
                type_annotation: None,
                initializer: Some(number_expr(42.0)),
                is_exported: false,
                span: Span::default(),
            })],
        };

        let mut generator = Generator::new();
        let ctx = generator.generate(AstNode::Program(program));

        assert_eq!(
            ctx.opcodes,
            vec![
                OpCode::Constant(0),
                OpCode::DefineGlobal(1)
            ]
        );
        assert_eq!(
            ctx.constants,
            vec![
                Value::Number(42.0),
                Value::String("a".to_string())
            ]
        );
    }

    #[test]
    fn honors_module_declaration_and_exports() {
        // Модули теперь не генерируют опкоды - экспорты обрабатываются на этапе разрешения имен
        let program = Program {
            statements: vec![
                Statement::ModuleDecl(crate::ponos::ast::ModuleDecl {
                    name: "math".to_string(),
                    span: Span::default(),
                }),
                Statement::VarDecl(VarDecl {
                    name: "x".to_string(),
                    type_annotation: None,
                    initializer: Some(number_expr(1.0)),
                    is_exported: true,
                    span: Span::default(),
                }),
            ],
        };

        let mut generator = Generator::new();
        let ctx = generator.generate(AstNode::Program(program));

        // Теперь просто генерируется переменная, без опкодов модулей
        assert_eq!(
            ctx.opcodes,
            vec![
                OpCode::Constant(0),
                OpCode::DefineGlobal(1),
            ]
        );

        assert_eq!(
            ctx.constants,
            vec![
                Value::Number(1.0),
                Value::String("x".to_string())
            ]
        );
    }

    // Тесты импортов удалены, так как импорты теперь обрабатываются на этапе загрузки модулей

    #[test]
    fn generates_locals_in_function_body() {
        let statements = vec![
            Statement::VarDecl(VarDecl {
                name: "x".to_string(),
                type_annotation: None,
                initializer: Some(number_expr(1.0)),
                is_exported: false,
                span: Span::default(),
            }),
            Statement::Assignment(crate::ponos::ast::AssignmentStatement {
                target: crate::ponos::ast::AssignmentTarget::Identifier("x".to_string()),
                value: number_expr(2.0),
                span: Span::default(),
            }),
            Statement::Expression(Expression::Identifier("x".to_string(), Span::default())),
        ];

        let mut generator = Generator::new();
        let ctx = generator.generate_function_body(statements);

        assert_eq!(
            ctx.opcodes,
            vec![
                OpCode::Constant(0),
                OpCode::DefineLocal(0),
                OpCode::Constant(1),
                OpCode::SetLocal(0),
                OpCode::GetLocal(0),
            ]
        );

        assert_eq!(
            ctx.constants,
            vec![Value::Number(1.0), Value::Number(2.0)]
        );
    }
}
