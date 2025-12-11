use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::ponos::opcode::OpCode;
use ordered_float::OrderedFloat;

/// Ключ для словаря - может быть любым типом Value
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValueKey {
    Number(OrderedFloat<f64>),
    String(String),
    Boolean(bool),
}

impl ValueKey {
    pub fn from_value(v: &Value) -> Result<ValueKey, String> {
        match v {
            Value::Number(n) => Ok(ValueKey::Number(OrderedFloat(*n))),
            Value::String(s) => Ok(ValueKey::String(s.clone())),
            Value::Boolean(b) => Ok(ValueKey::Boolean(*b)),
            _ => Err(
                "Ключом словаря может быть только число, строка или булево значение".to_string(),
            ),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Value {
    Number(f64),
    String(String),
    Boolean(bool),
    Nil,
    Function(Rc<Function>),
    NativeFunction(NativeFnId),
    Closure(Rc<Closure>),
    Class(Rc<Class>),
    Instance(Rc<RefCell<Instance>>),
    BoundMethod(Rc<BoundMethod>),
    Range(Option<f64>, Option<f64>), // (start, end) для срезов
    Array(Rc<RefCell<Vec<Value>>>),  // Массив (изменяемый)
    Dict(Rc<RefCell<HashMap<ValueKey, Value>>>), // Словарь (изменяемый)
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeFnId(pub usize);

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct Closure {
    pub function: Function,
    pub upvalues: Vec<Rc<RefCell<Upvalue>>>,
}

#[derive(Clone, Debug)]
pub enum Upvalue {
    Open(usize),   // Индекс на стеке
    Closed(Value), // Закрытое значение
}

// Структуры для ООП

#[derive(Clone, Debug)]
pub struct Class {
    pub name: String,
    pub methods: HashMap<String, Rc<Function>>,
    pub fields: Vec<String>,
    pub parent: Option<Rc<Class>>, // Для фазы 2 (наследование)
}

impl Class {
    /// Найти метод в этом классе или родительских классах
    pub fn find_method(&self, name: &str) -> Option<Rc<Function>> {
        if let Some(method) = self.methods.get(name) {
            Some(method.clone())
        } else if let Some(parent) = &self.parent {
            parent.find_method(name)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct Instance {
    pub class: Rc<Class>,
    pub fields: HashMap<String, Value>,
}

#[derive(Clone, Debug)]
pub struct BoundMethod {
    pub receiver: Rc<RefCell<Instance>>,
    pub method: Rc<Function>,
}

pub fn is_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Boolean(x), Value::Boolean(y)) => x == y,
        (Value::Nil, Value::Nil) => true,
        (Value::Range(s1, e1), Value::Range(s2, e2)) => s1 == s2 && e1 == e2,
        // Instance сравнивается по ссылке (идентичность объектов)
        (Value::Instance(a), Value::Instance(b)) => Rc::ptr_eq(a, b),
        // Массивы сравниваются поэлементно
        (Value::Array(a1), Value::Array(a2)) => {
            let arr1 = a1.borrow();
            let arr2 = a2.borrow();
            arr1.len() == arr2.len() && arr1.iter().zip(arr2.iter()).all(|(x, y)| is_equal(x, y))
        }
        // Словари сравниваются поэлементно
        (Value::Dict(d1), Value::Dict(d2)) => {
            let dict1 = d1.borrow();
            let dict2 = d2.borrow();
            dict1.len() == dict2.len()
                && dict1
                    .iter()
                    .all(|(k, v)| dict2.get(k).map_or(false, |v2| is_equal(v, v2)))
        }
        _ => false,
    }
}

// Реализация PartialEq для Value
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::Range(s1, e1), Value::Range(s2, e2)) => s1 == s2 && e1 == e2,
            (Value::Function(a), Value::Function(b)) => Rc::ptr_eq(a, b),
            (Value::Closure(a), Value::Closure(b)) => Rc::ptr_eq(a, b),
            (Value::Class(a), Value::Class(b)) => Rc::ptr_eq(a, b),
            (Value::Instance(a), Value::Instance(b)) => Rc::ptr_eq(a, b),
            (Value::BoundMethod(a), Value::BoundMethod(b)) => Rc::ptr_eq(a, b),
            (Value::NativeFunction(a), Value::NativeFunction(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => Rc::ptr_eq(a, b),
            (Value::Dict(a), Value::Dict(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
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
