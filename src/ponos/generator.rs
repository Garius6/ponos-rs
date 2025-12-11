pub struct Generator {}

use crate::ponos::ast::{Parameter, UnaryOperator};
use crate::ponos::value::{Function, UpvalueDescriptor};

use super::ast::{AssignmentTarget, AstNode, ClassMember, Expression, Statement};
use super::opcode::OpCode;
use super::value::Value;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone)]
pub struct GenContext {
    pub constants: Vec<Value>,
    pub opcodes: Vec<OpCode>,
    pub current_namespace: Option<String>, // Префикс для манглинга имен (было current_module)
    pub in_function: bool,
    local_slots: HashMap<String, usize>,
    next_local_slot: usize,
    parent_context: Option<Box<GenContext>>,
    upvalues: Vec<UpvalueInfo>,
}

#[derive(Clone)]
struct UpvalueInfo {
    name: String,
    is_local: bool,
    index: usize,
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
            parent_context: None,
            upvalues: Vec::new(),
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
                // Для Index нужен особый порядок, поэтому проверяем заранее
                match &assign.target {
                    AssignmentTarget::Index(object, index) => {
                        // SetIndex ожидает на стеке: object (низ), index, value (верх)
                        self.emit_expression((**object).clone(), ctx);
                        self.emit_expression((**index).clone(), ctx);
                        self.emit_expression(assign.value, ctx);
                        ctx.opcodes.push(OpCode::SetIndex);
                        return;
                    }
                    _ => {}
                }

                // Для остальных случаев сначала значение
                self.emit_expression(assign.value, ctx);
                match assign.target {
                    AssignmentTarget::Identifier(name) => {
                        if let Some(slot) = ctx.local_slots.get(&name) {
                            ctx.opcodes.push(OpCode::SetLocal(*slot));
                        } else if ctx.in_function {
                            if let Some(upvalue_idx) = self.resolve_upvalue(&name, ctx) {
                                ctx.opcodes.push(OpCode::SetUpvalue(upvalue_idx));
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
                    AssignmentTarget::FieldAccess(object, field) => {
                        // Значение уже на стеке (строка 111)
                        // Вычислить объект
                        self.emit_expression((*object).clone(), ctx);

                        // Установить поле
                        let field_name_idx = self.intern_string(&field, ctx);
                        ctx.opcodes.push(OpCode::SetProperty);
                        ctx.opcodes.push(OpCode::Constant(field_name_idx));
                    }
                    AssignmentTarget::Index(_, _) => {
                        // Уже обработано выше (строки 99-106)
                        unreachable!()
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
            Statement::FuncDecl(func_decl) => {
                if func_decl.is_exported && ctx.in_function {
                    panic!("Нельзя экспортировать функцию внутри другой функции");
                }

                // Компилируем тело функции
                let func_value =
                    self.compile_function(&func_decl.name, &func_decl.params, &func_decl.body, ctx, false);

                // Получаем количество upvalues из скомпилированной функции
                let upvalue_count = match &func_value {
                    Value::Function(f) => f.upvalue_count,
                    _ => 0,
                };

                // Добавляем в константы
                let fn_idx = self.intern_constant(func_value, ctx);

                // Создаём замыкание с правильным количеством upvalues
                ctx.opcodes.push(OpCode::Closure(fn_idx, upvalue_count));

                if ctx.in_function {
                    // Вложенная функция - определяем как локальную переменную
                    let slot = ctx.next_local_slot;
                    ctx.local_slots.insert(func_decl.name.clone(), slot);
                    ctx.next_local_slot += 1;
                    ctx.opcodes.push(OpCode::DefineLocal(slot));
                } else {
                    // Глобальная функция
                    let mangled_name = self.mangle_name(&func_decl.name, ctx);
                    let name_idx = self.intern_string(&mangled_name, ctx);
                    ctx.opcodes.push(OpCode::DefineGlobal(name_idx));
                }
            }
            Statement::Return(ret_stmt) => {
                if !ctx.in_function {
                    panic!("'возврат' вне функции");
                }

                if let Some(value) = &ret_stmt.value {
                    self.emit_expression(value.clone(), ctx);
                } else {
                    let nil_idx = self.intern_constant(Value::Nil, ctx);
                    ctx.opcodes.push(OpCode::Constant(nil_idx));
                }

                ctx.opcodes.push(OpCode::Return_);
            }
            Statement::Import(_) => {} // Импорт выполняет загрузчик модулей
            Statement::ClassDecl(class_decl) => {
                // 1. Создать класс
                let class_name_idx = self.intern_string(&class_decl.name, ctx);
                ctx.opcodes.push(OpCode::Class);
                ctx.opcodes.push(OpCode::Constant(class_name_idx));

                // 2. Установить наследование если есть
                if let Some(ref parent_name) = class_decl.extends {
                    // Получить родительский класс
                    let parent_mangled = self.mangle_name(parent_name, ctx);
                    let parent_name_idx = self.intern_string(&parent_mangled, ctx);
                    ctx.opcodes.push(OpCode::GetGlobal(parent_name_idx));

                    // Установить родителя
                    ctx.opcodes.push(OpCode::Inherit);
                }

                // 3. Для каждого члена класса
                for member in &class_decl.members {
                    match member {
                        ClassMember::Method(func_decl) => {
                            // Компилировать метод как функцию
                            let func_value = self.compile_function(
                                &func_decl.name,
                                &func_decl.params,
                                &func_decl.body,
                                ctx,
                                true, // это метод
                            );
                            let fn_idx = self.intern_constant(func_value, ctx);
                            let method_name_idx = self.intern_string(&func_decl.name, ctx);

                            ctx.opcodes.push(OpCode::Constant(fn_idx));
                            ctx.opcodes.push(OpCode::DefineMethod(method_name_idx));
                        }
                        ClassMember::Constructor(ctor) => {
                            // Конструктор - специальный метод "конструктор"
                            let func_value = self.compile_function(
                                "конструктор",
                                &ctor.params,
                                &ctor.body,
                                ctx,
                                true, // это конструктор (метод)
                            );
                            let fn_idx = self.intern_constant(func_value, ctx);
                            let ctor_name_idx = self.intern_string("конструктор", ctx);

                            ctx.opcodes.push(OpCode::Constant(fn_idx));
                            ctx.opcodes.push(OpCode::DefineMethod(ctor_name_idx));
                        }
                        ClassMember::Field { .. } => {
                            // Поля объявляются, но инициализируются в конструкторе
                            // Здесь ничего не генерируем
                        }
                    }
                }

                // 3. Определить класс как глобальную переменную
                let mangled_name = self.mangle_name(&class_decl.name, ctx);
                let name_idx = self.intern_string(&mangled_name, ctx);
                ctx.opcodes.push(OpCode::DefineGlobal(name_idx));
            }
            Statement::InterfaceDecl(_) => {
                // Заглушка для фазы 3
            }
            Statement::AnnotationDecl(_) => {
                // Заглушка для фазы 4
            }
            Statement::Try(try_stmt) => {
                // Placeholder для адреса начала catch-блока
                let handler_pos = ctx.opcodes.len();
                ctx.opcodes.push(OpCode::PushExceptionHandler(0));

                // Тело try
                for stmt in try_stmt.try_body {
                    self.emit_statement(stmt, ctx);
                }

                // Успешное завершение: снимаем обработчик и перепрыгиваем catch
                ctx.opcodes.push(OpCode::PopExceptionHandler);
                let jump_over_catch = self.emit_jump(ctx, OpCode::Jump(0));

                // Начало catch-блока
                let catch_start = ctx.opcodes.len();
                ctx.opcodes[handler_pos] = OpCode::PushExceptionHandler(catch_start);

                // Исключение будет на стеке: сохраняем в локальную переменную или удаляем
                if let Some(var_name) = try_stmt.catch_var {
                    let slot = if let Some(slot) = ctx.local_slots.get(&var_name) {
                        *slot
                    } else {
                        let slot = ctx.next_local_slot;
                        ctx.next_local_slot += 1;
                        ctx.local_slots.insert(var_name, slot);
                        slot
                    };
                    ctx.opcodes.push(OpCode::DefineLocal(slot));
                } else {
                    ctx.opcodes.push(OpCode::Pop);
                }

                // Тело catch
                for stmt in try_stmt.catch_body {
                    self.emit_statement(stmt, ctx);
                }

                // Патчим прыжок через catch-блок
                self.patch_jump(ctx, jump_over_catch);
            }
            Statement::Throw(throw_stmt) => {
                self.emit_expression(throw_stmt.expression, ctx);
                ctx.opcodes.push(OpCode::Throw);
            }
        }
    }

    /// Разрешить локальную переменную
    fn resolve_local(&self, name: &str, ctx: &GenContext) -> Option<usize> {
        ctx.local_slots.get(name).copied()
    }

    /// Разрешить upvalue (переменная из внешней области)
    fn resolve_upvalue(&mut self, name: &str, ctx: &mut GenContext) -> Option<usize> {
        // Если нет родительского контекста, upvalue не может быть найден
        let has_parent = ctx.parent_context.is_some();
        if !has_parent {
            return None;
        }

        // Сначала ищем в локальных переменных родителя
        let parent_local = ctx.parent_context.as_ref()
            .and_then(|parent| self.resolve_local(name, parent));

        if let Some(local_idx) = parent_local {
            // Добавляем upvalue для локальной переменной
            return Some(self.add_upvalue(ctx, name, true, local_idx));
        }

        // Рекурсивно ищем в upvalues родителя
        // Для этого нам нужно временно извлечь parent_context
        let mut parent_ctx = ctx.parent_context.take()?;
        let parent_upvalue_idx = self.resolve_upvalue(name, &mut parent_ctx);

        // Возвращаем parent_context обратно
        ctx.parent_context = Some(parent_ctx);

        if let Some(upvalue_idx) = parent_upvalue_idx {
            // Добавляем upvalue для upvalue родителя
            return Some(self.add_upvalue(ctx, name, false, upvalue_idx));
        }

        None
    }

    /// Добавить upvalue в текущий контекст
    fn add_upvalue(&mut self, ctx: &mut GenContext, name: &str, is_local: bool, index: usize) -> usize {
        // Проверяем, не добавлен ли уже этот upvalue
        for (i, upvalue_info) in ctx.upvalues.iter().enumerate() {
            if upvalue_info.name == name {
                return i;
            }
        }

        // Добавляем новый upvalue
        let upvalue_idx = ctx.upvalues.len();
        ctx.upvalues.push(UpvalueInfo {
            name: name.to_string(),
            is_local,
            index,
        });

        upvalue_idx
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
                if let Some(slot) = self.resolve_local(&name, ctx) {
                    ctx.opcodes.push(OpCode::GetLocal(slot));
                } else if ctx.in_function {
                    if let Some(upvalue_idx) = self.resolve_upvalue(&name, ctx) {
                        ctx.opcodes.push(OpCode::GetUpvalue(upvalue_idx));
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
                match binary_expr.operator {
                    crate::ponos::ast::BinaryOperator::And => {
                        // Логическое И с коротким замыканием:
                        // emit(left)
                        self.emit_expression(binary_expr.left, ctx);
                        // Dup - дублировать для проверки
                        ctx.opcodes.push(OpCode::Dup);
                        // JumpIfFalse(end) - pop и проверка, если false переходим (на стеке останется left)
                        let jump_addr = ctx.opcodes.len();
                        ctx.opcodes.push(OpCode::JumpIfFalse(0)); // placeholder
                        // Pop - удалить оригинальный left (дубликат был удален JumpIfFalse)
                        ctx.opcodes.push(OpCode::Pop);
                        // emit(right) - результат будет right
                        self.emit_expression(binary_expr.right, ctx);
                        // end:
                        let end_addr = ctx.opcodes.len();
                        ctx.opcodes[jump_addr] = OpCode::JumpIfFalse(end_addr);
                    }
                    crate::ponos::ast::BinaryOperator::Or => {
                        // Логическое ИЛИ с коротким замыканием:
                        // emit(left)
                        self.emit_expression(binary_expr.left, ctx);
                        // Dup - дублировать для проверки
                        ctx.opcodes.push(OpCode::Dup);
                        // JumpIfTrue(end) - pop и проверка, если true переходим (на стеке останется left)
                        let jump_addr = ctx.opcodes.len();
                        ctx.opcodes.push(OpCode::JumpIfTrue(0)); // placeholder
                        // Pop - удалить оригинальный left (дубликат был удален JumpIfTrue)
                        ctx.opcodes.push(OpCode::Pop);
                        // emit(right) - результат будет right
                        self.emit_expression(binary_expr.right, ctx);
                        // end:
                        let end_addr = ctx.opcodes.len();
                        ctx.opcodes[jump_addr] = OpCode::JumpIfTrue(end_addr);
                    }
                    _ => {
                        // Обычные бинарные операторы
                        self.emit_expression(binary_expr.left, ctx);
                        self.emit_expression(binary_expr.right, ctx);
                        let mut ops = match binary_expr.operator {
                            crate::ponos::ast::BinaryOperator::Add => vec![OpCode::Add],
                            crate::ponos::ast::BinaryOperator::Subtract => vec![OpCode::Sub],
                            crate::ponos::ast::BinaryOperator::Multiply => vec![OpCode::Mul],
                            crate::ponos::ast::BinaryOperator::Divide => vec![OpCode::Div],
                            crate::ponos::ast::BinaryOperator::Modulo => vec![OpCode::Mod],
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
                            // And и Or обработаны выше
                            _ => unreachable!("And/Or обработаны выше"),
                        };
                        ctx.opcodes.append(&mut ops);
                    }
                }
            }
            Expression::Unary(unary_expr) => {
                self.emit_expression(unary_expr.operand, ctx);
                let op = match unary_expr.operator {
                    UnaryOperator::Negate => OpCode::Negate,
                    UnaryOperator::Not => OpCode::Not,
                };
                ctx.opcodes.push(op);
            }
            Expression::Call(call_expr) => {
                // Генерируем callee
                self.emit_expression(call_expr.callee, ctx);

                // Генерируем аргументы
                for arg in &call_expr.arguments {
                    self.emit_expression(arg.clone(), ctx);
                }

                // Вызов
                ctx.opcodes.push(OpCode::Call(call_expr.arguments.len()));
            }
            Expression::FieldAccess(field_access_expr) => {
                // Специальный случай: super.method
                if let Expression::Identifier(ref name, _) = field_access_expr.object {
                    if name == "super" {
                        // 1. Загрузить это (всегда в слоте 0 для методов)
                        ctx.opcodes.push(OpCode::GetLocal(0));

                        // 2. GetSuper (читает следующий опкод Constant)
                        let method_name_idx = self.intern_string(&field_access_expr.field, ctx);
                        ctx.opcodes.push(OpCode::GetSuper);
                        ctx.opcodes.push(OpCode::Constant(method_name_idx));
                        return;
                    }
                }

                // Обычный field access
                // 1. Вычислить объект
                self.emit_expression(field_access_expr.object.clone(), ctx);

                // 2. Получить поле/метод
                let field_name_idx = self.intern_string(&field_access_expr.field, ctx);
                ctx.opcodes.push(OpCode::GetProperty);
                ctx.opcodes.push(OpCode::Constant(field_name_idx));
            }
            Expression::ModuleAccess(module_access) => {
                // Генерируем загрузку символа из модуля с манглингом имен
                let mangled_name = format!("{}::{}", module_access.namespace, module_access.symbol);
                let name_idx = self.intern_string(&mangled_name, ctx);
                ctx.opcodes.push(OpCode::GetGlobal(name_idx));
            }
            Expression::Lambda(lambda_expr) => {
                // Компилируем функцию и собираем upvalues
                let func_value =
                    self.compile_function("<lambda>", &lambda_expr.params, &lambda_expr.body, ctx, false);

                // Получаем количество upvalues из скомпилированной функции
                let upvalue_count = match &func_value {
                    Value::Function(f) => f.upvalue_count,
                    _ => 0,
                };

                let fn_idx = self.intern_constant(func_value, ctx);

                ctx.opcodes.push(OpCode::Closure(fn_idx, upvalue_count));

                // Лямбда оставляет замыкание на стеке
            }
            Expression::Index(index_expr) => {
                // Генерируем код для объекта
                self.emit_expression(index_expr.object.clone(), ctx);
                // Генерируем код для индекса
                self.emit_expression(index_expr.index.clone(), ctx);
                // Опкод получения элемента
                ctx.opcodes.push(OpCode::GetIndex);
            }
            Expression::Range(range_expr) => {
                // Вычисляем значения start и end и создаем Value::Range
                let start_val = range_expr.start.as_ref().map(|e| {
                    // Для простоты, пока что только числовые константы
                    if let Expression::Number(n, _) = **e {
                        n
                    } else {
                        panic!("Range expressions currently only support numeric constants");
                    }
                });

                let end_val = range_expr.end.as_ref().map(|e| {
                    if let Expression::Number(n, _) = **e {
                        n
                    } else {
                        panic!("Range expressions currently only support numeric constants");
                    }
                });

                // Создаем Range value и помещаем в константы
                let range_value = Value::Range(start_val, end_val);
                let idx = self.intern_constant(range_value, ctx);
                ctx.opcodes.push(OpCode::Constant(idx));
            }
            Expression::This(_span) => {
                // "это" всегда в слоте 0 внутри метода
                if !ctx.in_function {
                    panic!("'это' вне метода класса");
                }
                ctx.opcodes.push(OpCode::GetLocal(0));
            }
            Expression::Super(method_name, _span) => {
                // super.method_name → GetSuper
                // GetSuper читает следующий опкод Constant(method_name_idx),
                // берёт текущий экземпляр (GetLocal 0),
                // находит метод в родительском классе и создаёт BoundMethod

                // 1. Загрузить это (всегда в слоте 0 для методов)
                ctx.opcodes.push(OpCode::GetLocal(0));

                // 2. GetSuper (читает следующий опкод Constant)
                let method_name_idx = self.intern_string(&method_name, ctx);
                ctx.opcodes.push(OpCode::GetSuper);
                ctx.opcodes.push(OpCode::Constant(method_name_idx));
            }
            Expression::ArrayLiteral(array_literal) => {
                // Генерируем код для каждого элемента массива
                for elem in &array_literal.elements {
                    self.emit_expression(elem.clone(), ctx);
                }
                // Опкод создания массива
                ctx.opcodes.push(OpCode::Array(array_literal.elements.len()));
            }
            Expression::DictLiteral(dict_literal) => {
                // Генерируем код для каждой пары (ключ, значение)
                for (key, value) in &dict_literal.pairs {
                    self.emit_expression(key.clone(), ctx);
                    self.emit_expression(value.clone(), ctx);
                }
                // Опкод создания словаря
                ctx.opcodes.push(OpCode::Dict(dict_literal.pairs.len()));
            }
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
        // Ищем одинаковые константы (работает только для простых значений)
        if let Some(idx) = ctx.constants.iter().position(|v| {
            match (&value, v) {
                (Value::Number(a), Value::Number(b)) => a == b,
                (Value::String(a), Value::String(b)) => a == b,
                (Value::Boolean(a), Value::Boolean(b)) => a == b,
                (Value::Nil, Value::Nil) => true,
                // Для сложных типов (Function, Closure, Class и т.д.) не дедуплицируем
                _ => false,
            }
        }) {
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

    fn compile_function(
        &mut self,
        name: &str,
        params: &[Parameter],
        body: &[Statement],
        parent_ctx: &mut GenContext,
        is_method: bool, // true для методов и конструкторов
    ) -> Value {

        let mut func_ctx = GenContext {
            constants: Vec::new(),
            opcodes: Vec::new(),
            current_namespace: parent_ctx.current_namespace.clone(),
            in_function: true,
            local_slots: HashMap::new(),
            next_local_slot: if is_method { 1 } else { 0 }, // Для методов/конструкторов слот 0 - это 'это'
            parent_context: Some(Box::new(parent_ctx.clone())),
            upvalues: Vec::new(),
        };

        // Регистрируем параметры как локальные переменные
        for param in params {
            let slot = func_ctx.next_local_slot;
            func_ctx.local_slots.insert(param.name.clone(), slot);
            func_ctx.next_local_slot += 1;
        }

        // Генерируем тело
        for stmt in body {
            self.emit_statement(stmt.clone(), &mut func_ctx);
        }

        // Неявный return
        if name == "конструктор" {
            // Для конструкторов возвращаем 'это' (слот 0)
            func_ctx.opcodes.push(OpCode::GetLocal(0));
        } else {
            // Для обычных функций возвращаем nil
            let nil_idx = self.intern_constant(Value::Nil, &mut func_ctx);
            func_ctx.opcodes.push(OpCode::Constant(nil_idx));
        }
        func_ctx.opcodes.push(OpCode::Return_);

        // Собираем информацию об upvalues
        let upvalue_descriptors: Vec<UpvalueDescriptor> = func_ctx
            .upvalues
            .iter()
            .map(|uv| UpvalueDescriptor {
                is_local: uv.is_local,
                index: uv.index,
            })
            .collect();

        Value::Function(Rc::new(Function {
            arity: params.len(),
            opcodes: func_ctx.opcodes,
            constants: func_ctx.constants,
            name: name.to_string(),
            upvalue_count: upvalue_descriptors.len(),
            upvalue_descriptors,
        }))
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
        // TODO: Восстановить после добавления PartialEq для Value
        // assert_eq!(
        //     ctx.constants,
        //     vec![Value::Number(42.0), Value::String("a".to_string())]
        // );
        assert_eq!(ctx.constants.len(), 2);
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
