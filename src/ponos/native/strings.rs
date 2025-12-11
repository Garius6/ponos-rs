use crate::ponos::value::Value;
use std::cell::RefCell;
use std::rc::Rc;

/// строки.разделить(строка, разделитель) -> Array
pub fn str_split(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("разделить требует 2 аргумента: строка и разделитель".to_string());
    }

    let string = match &args[0] {
        Value::String(s) => s,
        _ => return Err("Первый аргумент должен быть строкой".to_string()),
    };

    let separator = match &args[1] {
        Value::String(s) => s,
        _ => return Err("Разделитель должен быть строкой".to_string()),
    };

    let parts: Vec<Value> = string
        .split(separator.as_str())
        .map(|s| Value::String(s.to_string()))
        .collect();

    Ok(Value::Array(Rc::new(RefCell::new(parts))))
}

/// строки.обрезать(строка) -> String (убрать пробелы с краёв)
pub fn str_trim(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("обрезать требует 1 аргумент".to_string());
    }

    match &args[0] {
        Value::String(s) => Ok(Value::String(s.trim().to_string())),
        _ => Err("Аргумент должен быть строкой".to_string()),
    }
}

/// строки.заменить(строка, что, на_что) -> String
pub fn str_replace(args: &[Value]) -> Result<Value, String> {
    if args.len() != 3 {
        return Err("заменить требует 3 аргумента".to_string());
    }

    let string = match &args[0] {
        Value::String(s) => s,
        _ => return Err("Первый аргумент должен быть строкой".to_string()),
    };

    let from = match &args[1] {
        Value::String(s) => s,
        _ => return Err("Второй аргумент должен быть строкой".to_string()),
    };

    let to = match &args[2] {
        Value::String(s) => s,
        _ => return Err("Третий аргумент должен быть строкой".to_string()),
    };

    Ok(Value::String(string.replace(from, to)))
}

/// строки.верхний_регистр(строка) -> String
pub fn str_to_upper(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("верхний_регистр требует 1 аргумент".to_string());
    }

    match &args[0] {
        Value::String(s) => Ok(Value::String(s.to_uppercase())),
        _ => Err("Аргумент должен быть строкой".to_string()),
    }
}

/// строки.нижний_регистр(строка) -> String
pub fn str_to_lower(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("нижний_регистр требует 1 аргумент".to_string());
    }

    match &args[0] {
        Value::String(s) => Ok(Value::String(s.to_lowercase())),
        _ => Err("Аргумент должен быть строкой".to_string()),
    }
}
