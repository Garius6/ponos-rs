use crate::ponos::{
    opcode::{self, OpCode},
    value::{self, Value},
};

pub struct VM {
    pub stack: Vec<Value>,
}

impl<'a> VM {
    pub fn new() -> Self {
        VM { stack: Vec::new() }
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
                opcode::OpCode::GetLocal => todo!(),
                opcode::OpCode::SetLocal => todo!(),
                opcode::OpCode::Closure => todo!(),
                opcode::OpCode::GetUpvalue => todo!(),
                opcode::OpCode::SetUpvalue => todo!(),
                opcode::OpCode::CloseUpvalues => todo!(),
                opcode::OpCode::Jump => todo!(),
                opcode::OpCode::JumpIfTrue => todo!(),
                opcode::OpCode::JumpIfFalse => todo!(),
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
                opcode::OpCode::DefineGlobal => todo!(),
                opcode::OpCode::SetGlobal => todo!(),
                opcode::OpCode::GetGlobal => todo!(),
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
}
