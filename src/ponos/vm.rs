use std::collections::HashMap;

use crate::ponos::{
    opcode::{self},
    value::{self, Value},
};

pub struct VM {
    pub stack: Vec<Value>,
    locals: Vec<Value>,
    globals: HashMap<String, Value>, // Плоское пространство глобальных переменных
}

impl<'a> VM {
    pub fn new() -> Self {
        VM {
            stack: Vec::new(),
            locals: Vec::new(),
            globals: HashMap::new(),
        }
    }

    pub fn execute(&mut self, opcodes: Vec<opcode::OpCode>, constants: &mut Vec<Value>) {
        let mut ip: usize = 0;
        loop {
            let cur_opcode = opcodes[ip];

            match cur_opcode {
                opcode::OpCode::Constant(idx) => {
                    self.stack.push(constants[idx].clone());
                }
                opcode::OpCode::Negate => {
                    let a = match self.stack.pop().unwrap() {
                        Value::Number(n) => n,
                        _ => panic!("Operand not a number"),
                    };

                    self.stack.push(Value::Number(-a));
                }
                opcode::OpCode::Add => self.binary_number_op(|a, b| a + b),
                opcode::OpCode::Sub => self.binary_number_op(|a, b| a - b),
                opcode::OpCode::Mul => self.binary_number_op(|a, b| a * b),
                opcode::OpCode::Div => self.binary_number_op(|a, b| a / b),
                opcode::OpCode::True_ => self.stack.push(Value::Boolean(true)),
                opcode::OpCode::False_ => self.stack.push(Value::Boolean(false)),
                opcode::OpCode::Eql => self.binary_logical_op(|a, b| value::is_equal(&a, &b)),
                opcode::OpCode::Not => {
                    let val = match self.stack.pop().unwrap() {
                        Value::Boolean(b) => b,
                        _ => panic!("Значение не булевого типа"),
                    };

                    self.stack.push(Value::Boolean(!val));
                }
                opcode::OpCode::Greater => self.binary_logical_op(|a, b| value::is_greater(&a, &b)),
                opcode::OpCode::Less => self.binary_logical_op(|a, b| {
                    !value::is_greater(&a, &b) && !value::is_equal(&a, &b)
                }),
                opcode::OpCode::GetLocal(slot) => {
                    let value = self
                        .locals
                        .get(slot)
                        .unwrap_or_else(|| panic!("Локальная переменная в слоте {slot} не найдена"))
                        .clone();
                    self.stack.push(value);
                }
                opcode::OpCode::SetLocal(slot) => {
                    let value = self
                        .stack
                        .pop()
                        .expect("Стек пуст при присваивании локальной переменной");
                    let slot_ref = self.locals.get_mut(slot).unwrap_or_else(|| {
                        panic!("Локальная переменная в слоте {slot} не найдена")
                    });
                    *slot_ref = value;
                }
                opcode::OpCode::DefineLocal(slot) => {
                    let value = self
                        .stack
                        .pop()
                        .expect("Стек пуст при определении локальной переменной");
                    if slot >= self.locals.len() {
                        self.locals.resize(slot + 1, Value::Nil);
                    }
                    self.locals[slot] = value;
                }
                opcode::OpCode::Closure => todo!(),
                opcode::OpCode::GetUpvalue => todo!(),
                opcode::OpCode::SetUpvalue => todo!(),
                opcode::OpCode::CloseUpvalues => todo!(),
                opcode::OpCode::Jump(addr) => {
                    // Безусловный переход
                    ip = addr;
                    continue; // Пропускаем ip += 1 в конце цикла
                }
                opcode::OpCode::JumpIfTrue(addr) => {
                    let condition = self
                        .stack
                        .pop()
                        .expect("Стек пуст при проверке условия JumpIfTrue");
                    if let Value::Boolean(true) = condition {
                        ip = addr;
                        continue; // Пропускаем ip += 1 в конце цикла
                    }
                }
                opcode::OpCode::JumpIfFalse(addr) => {
                    let condition = self
                        .stack
                        .pop()
                        .expect("Стек пуст при проверке условия JumpIfFalse");
                    if let Value::Boolean(false) = condition {
                        ip = addr;
                        continue; // Пропускаем ip += 1 в конце цикла
                    }
                }
                opcode::OpCode::Call => todo!(),
                opcode::OpCode::Return_ => todo!(),
                opcode::OpCode::Pop => todo!(),
                opcode::OpCode::Push => todo!(),
                opcode::OpCode::Class => todo!(),
                opcode::OpCode::Instance => todo!(),
                opcode::OpCode::GetProperty => todo!(),
                opcode::OpCode::SetProperty => todo!(),
                opcode::OpCode::Invoke => todo!(),
                opcode::OpCode::GetSuper => todo!(),
                opcode::OpCode::DefineGlobal(name_idx) => {
                    let name = self.expect_string(constants, name_idx);
                    let value = self
                        .stack
                        .pop()
                        .expect("Стек пуст при определении глобальной переменной");

                    if self.globals.contains_key(&name) {
                        panic!("Глобальная переменная {name} уже определена");
                    }

                    self.globals.insert(name, value);
                }
                opcode::OpCode::SetGlobal(name_idx) => {
                    let name = self.expect_string(constants, name_idx);
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
                opcode::OpCode::GetGlobal(name_idx) => {
                    let name = self.expect_string(constants, name_idx);
                    let value = self
                        .globals
                        .get(&name)
                        .cloned()
                        .unwrap_or_else(|| panic!("Глобальная переменная {name} не найдена"));

                    self.stack.push(value);
                }
                opcode::OpCode::Halt => {}
            };

            ip += 1;
            if ip >= opcodes.len() {
                break;
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

    fn expect_string(&self, constants: &[Value], idx: usize) -> String {
        match &constants[idx] {
            Value::String(s) => s.clone(),
            _ => panic!("Ожидалась строка в пуле констант по индексу {idx}"),
        }
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
            opcode::OpCode::Constant(0),
            opcode::OpCode::DefineLocal(0),
            opcode::OpCode::Constant(1),
            opcode::OpCode::SetLocal(0),
            opcode::OpCode::GetLocal(0),
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
            opcode::OpCode::Constant(1),
            opcode::OpCode::DefineGlobal(0),
            opcode::OpCode::GetGlobal(0),
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
            opcode::OpCode::Constant(1),
            opcode::OpCode::DefineGlobal(0),
            opcode::OpCode::GetGlobal(0),
        ];

        vm.execute(opcodes, &mut constants);

        assert_eq!(vm.stack.len(), 1);
        assert_eq!(vm.stack[0], Value::Number(3.14));
        assert_eq!(vm.globals["мат::ПИ"], Value::Number(3.14));
    }
}
