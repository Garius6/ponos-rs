use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::ponos::{
    opcode::OpCode,
    value::{self, BoundMethod, Class, Closure, Function, Instance, NativeFnId, Upvalue, Value},
};

#[derive(Debug)]
struct CallFrame {
    opcodes: Vec<OpCode>,
    constants: Vec<Value>,
    ip: usize,
    base: usize, // Базовый индекс в стеке
    upvalues: Vec<Rc<RefCell<Upvalue>>>,
}

type NativeFn = fn(&[Value]) -> Result<Value, String>;

pub struct VM {
    pub stack: Vec<Value>,
    globals: HashMap<String, Value>, // Плоское пространство глобальных переменных
    frames: Vec<CallFrame>,
    native_functions: Vec<NativeFn>,
    open_upvalues: Vec<Rc<RefCell<Upvalue>>>,
}

impl<'a> VM {
    pub fn new() -> Self {
        VM {
            stack: Vec::new(),
            globals: HashMap::new(),
            frames: Vec::new(),
            native_functions: Vec::new(),
            open_upvalues: Vec::new(),
        }
    }

    pub fn execute(&mut self, opcodes: Vec<OpCode>, constants: &mut Vec<Value>) {
        self.frames.push(CallFrame {
            opcodes,
            constants: constants.clone(),
            ip: 0,
            base: 0,
            upvalues: Vec::new(),
        });

        while !self.frames.is_empty() {
            let frame_idx = self.frames.len() - 1;

            if self.frames[frame_idx].ip >= self.frames[frame_idx].opcodes.len() {
                // Автоматический return
                if self.frames.len() == 1 {
                    // Для main frame очистить локальные переменные, оставив только результат
                    let base = self.frames[0].base;
                    if self.stack.len() > base {
                        let result = self.stack.pop().unwrap();
                        self.stack.truncate(base);
                        self.stack.push(result);
                    }
                    break;
                }
                self.stack.push(Value::Nil);
                let base = self.frames[frame_idx].base;
                self.frames.pop();
                self.stack.truncate(base);
                continue;
            }

            let cur_opcode = self.frames[frame_idx].opcodes[self.frames[frame_idx].ip];

            match cur_opcode {
                OpCode::Constant(idx) => {
                    self.stack
                        .push(self.frames[frame_idx].constants[idx].clone());
                }
                OpCode::Pop => {
                    self.stack.pop().expect("Стек пуст при Pop");
                }
                OpCode::Dup => {
                    let value = self.stack.last().expect("Стек пуст при Dup").clone();
                    self.stack.push(value);
                }
                OpCode::Negate => {
                    let a = match self.stack.pop().unwrap() {
                        Value::Number(n) => n,
                        _ => panic!("Operand not a number"),
                    };

                    self.stack.push(Value::Number(-a));
                }
                OpCode::Add => self.binary_add_op(),
                OpCode::Sub => self.binary_number_op(|a, b| a - b),
                OpCode::Mul => self.binary_number_op(|a, b| a * b),
                OpCode::Div => self.binary_number_op(|a, b| a / b),
                OpCode::Mod => self.binary_number_op(|a, b| a % b),
                OpCode::True_ => self.stack.push(Value::Boolean(true)),
                OpCode::False_ => self.stack.push(Value::Boolean(false)),
                OpCode::Eql => self.binary_logical_op(|a, b| value::is_equal(&a, &b)),
                OpCode::Not => {
                    let val = match self.stack.pop().unwrap() {
                        Value::Boolean(b) => b,
                        _ => panic!("Значение не булевого типа"),
                    };

                    self.stack.push(Value::Boolean(!val));
                }
                OpCode::Greater => self.binary_logical_op(|a, b| value::is_greater(&a, &b)),
                OpCode::Less => self.binary_logical_op(|a, b| {
                    !value::is_greater(&a, &b) && !value::is_equal(&a, &b)
                }),
                OpCode::GetLocal(slot) => {
                    let index = self.frames[frame_idx].base + slot;
                    let value = self.stack.get(index).cloned().unwrap_or(Value::Nil);
                    self.stack.push(value);
                }
                OpCode::SetLocal(slot) => {
                    let value = self.stack.pop().expect("Стек пуст");
                    let index = self.frames[frame_idx].base + slot;

                    if index >= self.stack.len() {
                        self.stack.resize(index + 1, Value::Nil);
                    }
                    self.stack[index] = value;
                }
                OpCode::DefineLocal(slot) => {
                    // Pop значение с evaluation stack и сохранить в слот локальной переменной
                    let value = self.stack.pop().expect("Стек пуст");
                    let index = self.frames[frame_idx].base + slot;

                    // Убеждаемся, что в стеке достаточно места для всех локальных переменных
                    // Локальные переменные идут сразу после base
                    while self.stack.len() <= index {
                        self.stack.push(Value::Nil);
                    }
                    self.stack[index] = value;
                }
                OpCode::Closure(fn_const_idx, _upvalue_count) => {
                    // Получаем функцию из константного пула
                    let function = match &self.frames[frame_idx].constants[fn_const_idx] {
                        Value::Function(f) => f.clone(),
                        other => panic!(
                            "Ожидалась функция в константном пуле по индексу {}, но найдено: {:?}",
                            fn_const_idx, other
                        ),
                    };

                    // Создаем upvalues для замыкания на основе дескрипторов из функции
                    let mut upvalues = Vec::new();
                    for descriptor in &function.upvalue_descriptors {
                        if descriptor.is_local {
                            // Захватываем локальную переменную из текущего фрейма
                            let stack_index = self.frames[frame_idx].base + descriptor.index;

                            // Проверяем, не создан ли уже upvalue для этого слота
                            let upvalue = self.capture_upvalue(stack_index);
                            upvalues.push(upvalue);
                        } else {
                            // Захватываем upvalue из родительского замыкания
                            if descriptor.index >= self.frames[frame_idx].upvalues.len() {
                                panic!(
                                    "Попытка захватить upvalue[{}], но у текущего фрейма только {} upvalues. \
                                    Возможно, замыкание создано внутри обычной функции, а не замыкания.",
                                    descriptor.index,
                                    self.frames[frame_idx].upvalues.len()
                                );
                            }
                            let parent_upvalue =
                                self.frames[frame_idx].upvalues[descriptor.index].clone();
                            upvalues.push(parent_upvalue);
                        }
                    }

                    // Создаем замыкание
                    let closure = Closure {
                        function: (*function).clone(),
                        upvalues,
                    };

                    self.stack.push(Value::Closure(Rc::new(closure)));
                }
                OpCode::GetUpvalue(index) => {
                    let frame = &self.frames[frame_idx];
                    let upvalue = &frame.upvalues[index];

                    let value = match &*upvalue.borrow() {
                        Upvalue::Open(stack_idx) => self.stack[*stack_idx].clone(),
                        Upvalue::Closed(val) => val.clone(),
                    };

                    self.stack.push(value);
                }
                OpCode::SetUpvalue(index) => {
                    let value = self.stack.pop().expect("Стек пуст");
                    let frame = &self.frames[frame_idx];
                    let upvalue = &frame.upvalues[index];

                    let mut upval = upvalue.borrow_mut();
                    match &mut *upval {
                        Upvalue::Open(stack_idx) => {
                            self.stack[*stack_idx] = value;
                        }
                        Upvalue::Closed(val) => {
                            *val = value;
                        }
                    }
                }
                OpCode::CloseUpvalues(local_count) => {
                    let frame = &self.frames[frame_idx];
                    let threshold = frame.base + local_count;
                    self.close_upvalues_from(threshold);
                }
                OpCode::Jump(addr) => {
                    // Безусловный переход
                    self.frames[frame_idx].ip = addr;
                    continue;
                }
                OpCode::JumpIfTrue(addr) => {
                    let condition = self
                        .stack
                        .pop()
                        .expect("Стек пуст при проверке условия JumpIfTrue");
                    if let Value::Boolean(true) = condition {
                        self.frames[frame_idx].ip = addr;
                        continue;
                    }
                }
                OpCode::JumpIfFalse(addr) => {
                    let condition = self
                        .stack
                        .pop()
                        .expect("Стек пуст при проверке условия JumpIfFalse");
                    if let Value::Boolean(false) = condition {
                        self.frames[frame_idx].ip = addr;
                        continue;
                    }
                }
                OpCode::Call(arg_count) => {
                    let callee_idx = self.stack.len() - arg_count - 1;
                    let callee = self.stack[callee_idx].clone();

                    match callee {
                        Value::Class(class) => {
                            // Создание экземпляра класса
                            let instance = Instance {
                                class: class.clone(),
                                fields: HashMap::new(),
                            };
                            let instance_rc = Rc::new(RefCell::new(instance));

                            // Заменяем класс на экземпляр на стеке
                            self.stack[callee_idx] = Value::Instance(instance_rc.clone());

                            // Вызываем конструктор если есть
                            if let Some(ctor) = class.methods.get("конструктор") {
                                self.call_function(ctor.clone(), arg_count, true).unwrap();
                            } else {
                                // Нет конструктора - просто удаляем аргументы
                                self.stack.drain(callee_idx + 1..);
                            }
                        }
                        Value::BoundMethod(bound) => {
                            // Заменяем BoundMethod на экземпляр (это будет slot 0)
                            self.stack[callee_idx] = Value::Instance(bound.receiver.clone());
                            self.call_function(bound.method.clone(), arg_count, true)
                                .unwrap();
                        }
                        Value::Function(func) => {
                            self.call_function(func, arg_count, false).unwrap()
                        }
                        Value::Closure(closure) => self.call_closure(closure, arg_count).unwrap(),
                        Value::NativeFunction(id) => self.call_native(id, arg_count).unwrap(),
                        _ => panic!("Попытка вызвать не-функцию"),
                    }
                }
                OpCode::Return_ => {
                    let return_value = self.stack.pop().unwrap_or(Value::Nil);
                    let base = self.frames[frame_idx].base;

                    self.close_upvalues_from(base);
                    self.frames.pop().expect("Пустой стек вызовов");
                    self.stack.truncate(base);
                    self.stack.push(return_value);
                    continue;
                }
                OpCode::Class => {
                    // Следующий опкод: Constant с именем класса
                    self.frames[frame_idx].ip += 1;
                    let name_opcode = self.frames[frame_idx].opcodes[self.frames[frame_idx].ip];
                    let name = match name_opcode {
                        OpCode::Constant(idx) => match &self.frames[frame_idx].constants[idx] {
                            Value::String(s) => s.clone(),
                            _ => panic!("Имя класса должно быть строкой"),
                        },
                        _ => panic!("Ожидался Constant после Class"),
                    };

                    let class = Class {
                        name,
                        methods: HashMap::new(),
                        fields: Vec::new(),
                        parent: None,
                    };

                    self.stack.push(Value::Class(Rc::new(class)));
                }
                OpCode::Inherit => {
                    // Стек: [subclass, superclass]
                    let superclass_value = self.stack.pop().unwrap();
                    let superclass = match superclass_value {
                        Value::Class(c) => c,
                        _ => panic!("Inherit: суперкласс должен быть классом"),
                    };

                    // Получаем подкласс (остается на стеке)
                    let subclass_value = self.stack.last_mut().unwrap();
                    match subclass_value {
                        Value::Class(subclass_rc) => {
                            let subclass_mut = Rc::make_mut(subclass_rc);
                            subclass_mut.parent = Some(superclass);
                        }
                        _ => panic!("Inherit: подкласс должен быть классом"),
                    }
                }
                OpCode::DefineMethod(name_idx) => {
                    let method_name =
                        self.expect_string(&self.frames[frame_idx].constants, name_idx);
                    let method = self.stack.pop().unwrap();

                    let class_value = self.stack.last_mut().unwrap();
                    match class_value {
                        Value::Class(class_rc) => {
                            let class_mut = Rc::make_mut(class_rc);

                            let func = match method {
                                Value::Function(f) => f,
                                Value::Closure(c) => Rc::new(c.function.clone()),
                                _ => panic!("Метод должен быть функцией или замыканием"),
                            };

                            class_mut.methods.insert(method_name, func);
                        }
                        _ => panic!("DefineMethod: не класс на вершине стека"),
                    }
                }
                OpCode::GetProperty => {
                    // Следующий опкод: Constant с индексом имени свойства
                    self.frames[frame_idx].ip += 1;
                    let name_opcode = self.frames[frame_idx].opcodes[self.frames[frame_idx].ip];
                    let property_name = match name_opcode {
                        OpCode::Constant(idx) => match &self.frames[frame_idx].constants[idx] {
                            Value::String(s) => s.clone(),
                            _ => panic!("Имя свойства должно быть строкой"),
                        },
                        _ => panic!("Ожидался Constant после GetProperty"),
                    };

                    let instance_value = self.stack.pop().unwrap();
                    match instance_value {
                        Value::Instance(instance_rc) => {
                            // Сначала ищем в полях
                            let field_value =
                                instance_rc.borrow().fields.get(&property_name).cloned();

                            if let Some(value) = field_value {
                                self.stack.push(value);
                            }
                            // Потом в методах (включая родительские классы) → BoundMethod
                            else {
                                let method = instance_rc.borrow().class.find_method(&property_name);

                                if let Some(method) = method {
                                    let bound = BoundMethod {
                                        receiver: instance_rc.clone(),
                                        method,
                                    };
                                    self.stack.push(Value::BoundMethod(Rc::new(bound)));
                                } else {
                                    panic!("Свойство '{}' не найдено", property_name);
                                }
                            }
                        }
                        _ => panic!("GetProperty: не экземпляр, получен {:?}", instance_value),
                    }
                }
                OpCode::SetProperty => {
                    // Следующий опкод: Constant с индексом имени свойства
                    self.frames[frame_idx].ip += 1;
                    let name_opcode = self.frames[frame_idx].opcodes[self.frames[frame_idx].ip];
                    let property_name = match name_opcode {
                        OpCode::Constant(idx) => match &self.frames[frame_idx].constants[idx] {
                            Value::String(s) => s.clone(),
                            _ => panic!("Имя свойства должно быть строкой"),
                        },
                        _ => panic!("Ожидался Constant после SetProperty"),
                    };

                    let instance_value = self.stack.pop().unwrap();
                    let value = self.stack.pop().unwrap();

                    match instance_value {
                        Value::Instance(instance_rc) => {
                            instance_rc
                                .borrow_mut()
                                .fields
                                .insert(property_name, value.clone());
                            self.stack.push(value);
                        }
                        _ => panic!("SetProperty: не экземпляр"),
                    }
                }
                OpCode::GetSuper => {
                    // Следующий опкод: Constant с именем метода
                    self.frames[frame_idx].ip += 1;
                    let name_opcode = self.frames[frame_idx].opcodes[self.frames[frame_idx].ip];
                    let method_name = match name_opcode {
                        OpCode::Constant(idx) => match &self.frames[frame_idx].constants[idx] {
                            Value::String(s) => s.clone(),
                            _ => panic!("Имя метода должно быть строкой"),
                        },
                        _ => panic!("Ожидался Constant после GetSuper"),
                    };

                    // Получаем экземпляр со стека
                    let instance_value = self.stack.pop().unwrap();
                    match instance_value {
                        Value::Instance(instance_rc) => {
                            // Получаем родительский класс
                            let parent_class = instance_rc
                                .borrow()
                                .class
                                .parent
                                .as_ref()
                                .expect("super вызван в классе без родителя")
                                .clone();

                            // Ищем метод в родительском классе
                            let method = parent_class.find_method(&method_name).expect(&format!(
                                "Метод '{}' не найден в родительском классе",
                                method_name
                            ));

                            // Создаём BoundMethod
                            let bound = BoundMethod {
                                receiver: instance_rc.clone(),
                                method,
                            };

                            self.stack.push(Value::BoundMethod(Rc::new(bound)));
                        }
                        _ => panic!("GetSuper: ожидался экземпляр класса"),
                    }
                }
                OpCode::GetIndex => {
                    let index = self.stack.pop().unwrap();
                    let object = self.stack.pop().unwrap();

                    match (&object, &index) {
                        // Индексирование строки
                        (Value::String(s), Value::Number(n)) => {
                            let idx = *n as usize;
                            if idx >= s.chars().count() {
                                panic!(
                                    "Индекс {} вне диапазона для строки длиной {}",
                                    idx,
                                    s.chars().count()
                                );
                            }
                            let ch = s.chars().nth(idx).unwrap();
                            self.stack.push(Value::String(ch.to_string()));
                        }
                        // Срез строки
                        (Value::String(s), Value::Range(start, end)) => {
                            let char_count = s.chars().count();
                            let start_idx = start.unwrap_or(0.0) as usize;
                            let end_idx = end.unwrap_or(char_count as f64) as usize;

                            if start_idx > char_count || end_idx > char_count || start_idx > end_idx
                            {
                                panic!(
                                    "Некорректные границы среза: [{}:{}] для строки длиной {}",
                                    start_idx, end_idx, char_count
                                );
                            }

                            let slice: String = s
                                .chars()
                                .skip(start_idx)
                                .take(end_idx - start_idx)
                                .collect();
                            self.stack.push(Value::String(slice));
                        }
                        _ => panic!("Индексирование не поддерживается для данных типов"),
                    }
                }
                OpCode::DefineGlobal(name_idx) => {
                    let name = self.expect_string(&self.frames[frame_idx].constants, name_idx);
                    let value = self
                        .stack
                        .pop()
                        .expect("Стек пуст при определении глобальной переменной");

                    if self.globals.contains_key(&name) {
                        panic!("Глобальная переменная {name} уже определена");
                    }

                    self.globals.insert(name, value);
                }
                OpCode::SetGlobal(name_idx) => {
                    let name = self.expect_string(&self.frames[frame_idx].constants, name_idx);
                    let value = self
                        .stack
                        .pop()
                        .expect("Стек пуст при присваивании глобальной переменной");

                    let slot = self
                        .globals
                        .get_mut(&name)
                        .unwrap_or_else(|| panic!("Глобальная переменная {name} не найдена"));

                    *slot = value;
                }
                OpCode::GetGlobal(name_idx) => {
                    let name = self.expect_string(&self.frames[frame_idx].constants, name_idx);
                    let value = self
                        .globals
                        .get(&name)
                        .cloned()
                        .unwrap_or_else(|| panic!("Глобальная переменная {name} не найдена"));

                    self.stack.push(value);
                }
                OpCode::Halt => {}
            };

            if frame_idx < self.frames.len() {
                self.frames[frame_idx].ip += 1;
            }
        }
    }

    fn binary_logical_op<F>(&mut self, f: F)
    where
        F: FnOnce(Value, Value) -> bool,
    {
        let b = self.stack.pop().unwrap();
        let a = self.stack.pop().unwrap();

        self.stack.push(Value::Boolean(f(a, b)));
    }

    fn binary_number_op<F>(&mut self, f: F)
    where
        F: FnOnce(f64, f64) -> f64,
    {
        let b = match self.stack.pop().unwrap() {
            Value::Number(n) => n,
            _ => panic!("Right operand is not a number"),
        };

        let a = match self.stack.pop().unwrap() {
            Value::Number(n) => n,
            _ => panic!("Left operand is not a number"),
        };

        self.stack.push(Value::Number(f(a, b)));
    }

    fn binary_add_op(&mut self) {
        let right = self.stack.pop().unwrap();
        let left = self.stack.pop().unwrap();

        match (&left, &right) {
            // Конкатенация строк
            (Value::String(s1), Value::String(s2)) => {
                self.stack.push(Value::String(format!("{}{}", s1, s2)));
            }
            // Арифметика чисел
            (Value::Number(n1), Value::Number(n2)) => {
                self.stack.push(Value::Number(n1 + n2));
            }
            // Преобразование число + строка
            (Value::Number(n), Value::String(s)) => {
                self.stack.push(Value::String(format!("{}{}", n, s)));
            }
            (Value::String(s), Value::Number(n)) => {
                self.stack.push(Value::String(format!("{}{}", s, n)));
            }
            _ => panic!("Оператор + поддерживает только числа и строки"),
        }
    }

    fn expect_string(&self, constants: &[Value], idx: usize) -> String {
        match &constants[idx] {
            Value::String(s) => s.clone(),
            _ => panic!("Ожидалась строка в пуле констант по индексу {idx}"),
        }
    }

    fn call_function(
        &mut self,
        func: Rc<Function>,
        arg_count: usize,
        is_method: bool,
    ) -> Result<(), String> {
        if arg_count != func.arity {
            return Err(format!(
                "Ожидается {} аргументов, передано {}",
                func.arity, arg_count
            ));
        }

        let base = self.stack.len() - arg_count;

        // Для методов и конструкторов не удаляем callee - он станет 'это' (слот 0)
        let final_base = if !is_method {
            self.stack.remove(base - 1); // Удалить callee
            base - 1 // После удаления callee, base сдвигается
        } else {
            base - 1
        };

        self.frames.push(CallFrame {
            opcodes: func.opcodes.clone(),
            constants: func.constants.clone(),
            ip: 0,
            base: final_base,
            upvalues: Vec::new(),
        });

        Ok(())
    }

    fn call_closure(&mut self, closure: Rc<Closure>, arg_count: usize) -> Result<(), String> {
        if arg_count != closure.function.arity {
            return Err(format!(
                "Ожидается {} аргументов, передано {}",
                closure.function.arity, arg_count
            ));
        }

        let base = self.stack.len() - arg_count;
        self.stack.remove(base - 1); // Удалить callee
        // После удаления callee, base сдвигается на 1
        let final_base = base - 1;

        self.frames.push(CallFrame {
            opcodes: closure.function.opcodes.clone(),
            constants: closure.function.constants.clone(),
            ip: 0,
            base: final_base,
            upvalues: closure.upvalues.clone(),
        });

        Ok(())
    }

    fn call_native(&mut self, id: NativeFnId, arg_count: usize) -> Result<(), String> {
        let args_start = self.stack.len() - arg_count;
        let args: Vec<Value> = self.stack.drain(args_start..).collect();

        self.stack.pop(); // Удалить callee

        let native_fn = self.native_functions[id.0];
        let result = native_fn(&args)?;

        self.stack.push(result);
        Ok(())
    }

    fn capture_upvalue(&mut self, stack_index: usize) -> Rc<RefCell<Upvalue>> {
        // Ищем существующий открытый upvalue для этого индекса стека
        for upvalue in &self.open_upvalues {
            if let Upvalue::Open(idx) = *upvalue.borrow() {
                if idx == stack_index {
                    return upvalue.clone();
                }
            }
        }

        // Создаем новый открытый upvalue
        let new_upvalue = Rc::new(RefCell::new(Upvalue::Open(stack_index)));
        self.open_upvalues.push(new_upvalue.clone());
        new_upvalue
    }

    fn close_upvalues_from(&mut self, stack_index: usize) {
        for upvalue in &self.open_upvalues {
            let mut upval = upvalue.borrow_mut();
            if let Upvalue::Open(idx) = *upval {
                if idx >= stack_index {
                    *upval = Upvalue::Closed(self.stack[idx].clone());
                }
            }
        }
    }

    fn register_native(&mut self, func: NativeFn) -> NativeFnId {
        let id = NativeFnId(self.native_functions.len());
        self.native_functions.push(func);
        id
    }
    pub fn register_and_define(&mut self, name: &str, func: NativeFn) {
        let id = self.register_native(func);
        self.globals
            .insert(name.to_string(), Value::NativeFunction(id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defines_and_sets_locals() {
        let mut vm = VM::new();
        let mut constants = vec![Value::Number(1.0), Value::Number(2.0)];

        let opcodes = vec![
            OpCode::Constant(0),
            OpCode::DefineLocal(0),
            OpCode::Constant(1),
            OpCode::SetLocal(0),
            OpCode::GetLocal(0),
        ];

        vm.execute(opcodes, &mut constants);

        assert_eq!(vm.stack, vec![Value::Number(2.0)]);
    }

    #[test]
    fn defines_and_gets_global_variable() {
        let mut vm = VM::new();
        let mut constants = vec![
            Value::String("x".to_string()), // 0
            Value::Number(42.0),            // 1
        ];

        let opcodes = vec![
            OpCode::Constant(1),
            OpCode::DefineGlobal(0),
            OpCode::GetGlobal(0),
        ];

        vm.execute(opcodes, &mut constants);

        assert_eq!(vm.stack.len(), 1);
        assert_eq!(vm.stack[0], Value::Number(42.0));
    }

    #[test]
    fn module_variables_use_mangled_names() {
        let mut vm = VM::new();
        let mut constants = vec![
            Value::String("мат::ПИ".to_string()), // 0 - манглированное имя
            Value::Number(3.14),                  // 1
        ];

        let opcodes = vec![
            OpCode::Constant(1),
            OpCode::DefineGlobal(0),
            OpCode::GetGlobal(0),
        ];

        vm.execute(opcodes, &mut constants);

        assert_eq!(vm.stack.len(), 1);
        assert_eq!(vm.stack[0], Value::Number(3.14));
        assert_eq!(vm.globals["мат::ПИ"], Value::Number(3.14));
    }
}
