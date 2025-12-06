pub struct Generator {}

use crate::ponos::ast::UnaryOperator;
use crate::ponos::opcode;

use super::ast::{AssignmentTarget, AstNode, Expression, Statement};
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
        context.opcodes.push(OpCode::Halt);
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
            Statement::If(if_statement) => {
                self.emit_expression(if_statement.condition, ctx);
                let jmp_false = self.emit_jump(ctx, OpCode::JumpIfFalse(0));
                for stmt in if_statement.then_branch {
                    self.emit_statement(stmt, ctx);
                }

                let jmp_end = self.emit_jump(ctx, OpCode::Jump(0));
                self.patch_jump(ctx, jmp_false);
                match if_statement.else_branch {
                    Some(block) => {
                        for stmt in block {
                            self.emit_statement(stmt, ctx);
                        }
                    }
                    None => {}
                }
                self.patch_jump(ctx, jmp_end);
            }
            Statement::While(while_statement) => {
                let cond_pos = ctx.opcodes.len();
                self.emit_expression(while_statement.condition, ctx);
                let jmp_false = self.emit_jump(ctx, OpCode::JumpIfFalse(0));
                for stmt in while_statement.body {
                    self.emit_statement(stmt, ctx);
                }
                ctx.opcodes.push(OpCode::Jump(cond_pos));
                self.patch_jump(ctx, jmp_false);
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

    fn emit_jump(&self, ctx: &mut GenContext, opcode: OpCode) -> usize {
        // Добавляем опкод с placeholder адресом (0)
        let placeholder_opcode = match opcode {
            OpCode::Jump(_) => OpCode::Jump(0),
            OpCode::JumpIfTrue(_) => OpCode::JumpIfTrue(0),
            OpCode::JumpIfFalse(_) => OpCode::JumpIfFalse(0),
            _ => panic!("emit_jump вызван с неправильным опкодом"),
        };

        ctx.opcodes.push(placeholder_opcode);
        // Возвращаем индекс добавленного опкода
        ctx.opcodes.len() - 1
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

    fn patch_jump(&self, ctx: &mut GenContext, operand_pos: usize) {
        // Вычисляем адрес куда нужно прыгнуть (текущая позиция)
        let jump_target = ctx.opcodes.len();

        // Заменяем опкод на тот же тип, но с правильным адресом
        let patched_opcode = match ctx.opcodes[operand_pos] {
            OpCode::Jump(_) => OpCode::Jump(jump_target),
            OpCode::JumpIfTrue(_) => OpCode::JumpIfTrue(jump_target),
            OpCode::JumpIfFalse(_) => OpCode::JumpIfFalse(jump_target),
            _ => panic!("patch_jump вызван для не-jump опкода"),
        };

        ctx.opcodes[operand_pos] = patched_opcode;
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
            vec![OpCode::Constant(0), OpCode::DefineGlobal(1), OpCode::Halt]
        );
        assert_eq!(
            ctx.constants,
            vec![Value::Number(42.0), Value::String("a".to_string())]
        );
    }

    #[test]
    fn honors_module_declaration_and_exports() {
        // Экспорты обрабатываются на этапе разрешения имен
        let program = Program {
            statements: vec![Statement::VarDecl(VarDecl {
                name: "x".to_string(),
                type_annotation: None,
                initializer: Some(number_expr(1.0)),
                is_exported: true,
                span: Span::default(),
            })],
        };

        let mut generator = Generator::new();
        let ctx = generator.generate(AstNode::Program(program));

        // Теперь просто генерируется переменная, без опкодов модулей
        assert_eq!(
            ctx.opcodes,
            vec![OpCode::Constant(0), OpCode::DefineGlobal(1), OpCode::Halt]
        );

        assert_eq!(
            ctx.constants,
            vec![Value::Number(1.0), Value::String("x".to_string())]
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

        assert_eq!(ctx.constants, vec![Value::Number(1.0), Value::Number(2.0)]);
    }

    #[test]
    fn generates_if_statement_with_else() {
        use crate::ponos::ast::{BinaryExpr, BinaryOperator, IfStatement};

        // если x > 5 то
        //     y = 10
        // иначе
        //     y = 20
        // конец
        let condition = Expression::Binary(Box::new(BinaryExpr {
            left: Expression::Identifier("x".to_string(), Span::default()),
            operator: BinaryOperator::Greater,
            right: number_expr(5.0),
            span: Span::default(),
        }));

        let then_branch = vec![Statement::Assignment(
            crate::ponos::ast::AssignmentStatement {
                target: crate::ponos::ast::AssignmentTarget::Identifier("y".to_string()),
                value: number_expr(10.0),
                span: Span::default(),
            },
        )];

        let else_branch = vec![Statement::Assignment(
            crate::ponos::ast::AssignmentStatement {
                target: crate::ponos::ast::AssignmentTarget::Identifier("y".to_string()),
                value: number_expr(20.0),
                span: Span::default(),
            },
        )];

        let program = Program {
            statements: vec![Statement::If(IfStatement {
                condition,
                then_branch,
                else_branch: Some(else_branch),
                span: Span::default(),
            })],
        };

        let mut generator = Generator::new();
        let ctx = generator.generate(AstNode::Program(program));

        assert!(matches!(ctx.opcodes[3], OpCode::JumpIfFalse(7)));
        assert!(matches!(ctx.opcodes[6], OpCode::Jump(9)));
    }
}
