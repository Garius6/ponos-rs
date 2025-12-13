use crate::ponos::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Сигнатура встроенного метода: (receiver, args) -> Result<Value, String>
pub type BuiltinMethod = fn(&Value, &[Value]) -> Result<Value, String>;

/// Дискриминант типа для идентификации типа Value
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum TypeDiscriminant {
    Array,
    String,
    Dict,
}

/// Реестр встроенных методов для базовых типов
pub struct BuiltinMethodRegistry {
    methods: HashMap<(TypeDiscriminant, String), BuiltinMethod>,
}

impl BuiltinMethodRegistry {
    pub fn new() -> Self {
        let mut registry = BuiltinMethodRegistry {
            methods: HashMap::new(),
        };

        // Регистрация методов для массивов
        registry.register(TypeDiscriminant::Array, "добавить", array_push);
        registry.register(TypeDiscriminant::Array, "очистить", array_clear);

        // Регистрация методов для строк
        registry.register(TypeDiscriminant::String, "длина", string_length);
        registry.register(TypeDiscriminant::String, "разделить", string_split);

        // Регистрация методов для словарей
        registry.register(TypeDiscriminant::Dict, "ключи", dict_keys);
        registry.register(TypeDiscriminant::Dict, "значения", dict_values);
        registry.register(TypeDiscriminant::Dict, "очистить", dict_clear);

        registry
    }

    fn register(&mut self, type_disc: TypeDiscriminant, method_name: &str, method: BuiltinMethod) {
        self.methods.insert((type_disc, method_name.to_string()), method);
    }

    pub fn get(&self, type_disc: TypeDiscriminant, method_name: &str) -> Option<BuiltinMethod> {
        self.methods.get(&(type_disc, method_name.to_string())).copied()
    }
}

// ============================================================================
// Методы для массивов
// ============================================================================

/// Метод добавить(элемент) - добавляет элемент в конец массива
fn array_push(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("Метод 'добавить' принимает 1 аргумент, передано: {}", args.len()));
    }

    match receiver {
        Value::Array(arr) => {
            arr.borrow_mut().push(args[0].clone());
            Ok(Value::Nil) // Возвращаем Nil, так как метод изменяет массив на месте
        }
        _ => Err("Метод 'добавить' можно вызывать только на массиве".to_string()),
    }
}

/// Метод очистить() - очищает массив
fn array_clear(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!("Метод 'очистить' не принимает аргументы, передано: {}", args.len()));
    }

    match receiver {
        Value::Array(arr) => {
            arr.borrow_mut().clear();
            Ok(Value::Nil) // Возвращаем Nil, так как метод изменяет массив на месте
        }
        _ => Err("Метод 'очистить' можно вызывать только на массиве".to_string()),
    }
}

// ============================================================================
// Методы для строк
// ============================================================================

/// Метод длина() - возвращает длину строки
fn string_length(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!("Метод 'длина' не принимает аргументы, передано: {}", args.len()));
    }

    match receiver {
        Value::String(s) => Ok(Value::Number(s.chars().count() as f64)),
        _ => Err("Метод 'длина' можно вызывать только на строке".to_string()),
    }
}

/// Метод разделить(разделитель) - разбивает строку по разделителю
fn string_split(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err(format!("Метод 'разделить' принимает 1 аргумент, передано: {}", args.len()));
    }

    let separator = match &args[0] {
        Value::String(s) => s.as_str(),
        _ => return Err("Аргумент метода 'разделить' должен быть строкой".to_string()),
    };

    match receiver {
        Value::String(s) => {
            let parts: Vec<Value> = s
                .split(separator)
                .map(|part| Value::String(part.to_string()))
                .collect();
            Ok(Value::Array(Rc::new(RefCell::new(parts))))
        }
        _ => Err("Метод 'разделить' можно вызывать только на строке".to_string()),
    }
}

// ============================================================================
// Методы для словарей
// ============================================================================

/// Метод ключи() - возвращает массив ключей словаря
fn dict_keys(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!("Метод 'ключи' не принимает аргументы, передано: {}", args.len()));
    }

    match receiver {
        Value::Dict(dict) => {
            let dict_borrow = dict.borrow();
            let keys: Vec<Value> = dict_borrow
                .keys()
                .map(|k| match k {
                    crate::ponos::value::ValueKey::String(s) => Value::String(s.clone()),
                    crate::ponos::value::ValueKey::Number(n) => Value::Number(n.into_inner()),
                    crate::ponos::value::ValueKey::Boolean(b) => Value::Boolean(*b),
                })
                .collect();
            Ok(Value::Array(Rc::new(RefCell::new(keys))))
        }
        _ => Err("Метод 'ключи' можно вызывать только на словаре".to_string()),
    }
}

/// Метод значения() - возвращает массив значений словаря
fn dict_values(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!("Метод 'значения' не принимает аргументы, передано: {}", args.len()));
    }

    match receiver {
        Value::Dict(dict) => {
            let dict_borrow = dict.borrow();
            let values: Vec<Value> = dict_borrow.values().cloned().collect();
            Ok(Value::Array(Rc::new(RefCell::new(values))))
        }
        _ => Err("Метод 'значения' можно вызывать только на словаре".to_string()),
    }
}

/// Метод очистить() - очищает словарь
fn dict_clear(receiver: &Value, args: &[Value]) -> Result<Value, String> {
    if !args.is_empty() {
        return Err(format!("Метод 'очистить' не принимает аргументы, передано: {}", args.len()));
    }

    match receiver {
        Value::Dict(dict) => {
            dict.borrow_mut().clear();
            Ok(Value::Nil) // Возвращаем Nil, так как метод изменяет словарь на месте
        }
        _ => Err("Метод 'очистить' можно вызывать только на словаре".to_string()),
    }
}
