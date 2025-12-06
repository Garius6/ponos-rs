use std::{cell::RefCell, rc::Rc};

use crate::ponos::opcode::OpCode;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    String(String),
    Boolean(bool),
    Nil,
    Function(Rc<Function>),
    NativeFunction(NativeFnId),
    Closure(Rc<Closure>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeFnId(pub usize);

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub arity: usize,
    pub opcodes: Vec<OpCode>,
    pub constants: Vec<Value>,
    pub name: String,
    pub upvalue_count: usize,
    pub upvalue_descriptors: Vec<UpvalueDescriptor>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UpvalueDescriptor {
    pub is_local: bool, // true если захватывается локальная переменная, false если upvalue родителя
    pub index: usize,   // Индекс локальной переменной или upvalue
}

#[derive(Clone, Debug, PartialEq)]
pub struct Closure {
    pub function: Function,
    pub upvalues: Vec<Rc<RefCell<Upvalue>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Upvalue {
    Open(usize),   // Индекс на стеке
    Closed(Value), // Закрытое значение
}

pub fn is_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Boolean(x), Value::Boolean(y)) => x == y,
        (Value::Nil, Value::Nil) => true,
        _ => false,
    }
}

pub fn is_greater(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x > y,
        (Value::String(x), Value::String(y)) => x > y,
        (Value::Boolean(x), Value::Boolean(y)) => x > y,
        _ => false,
    }
}
