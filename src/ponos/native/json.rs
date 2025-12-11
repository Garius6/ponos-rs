use std::{cell::RefCell, collections::HashMap, rc::Rc};

use serde_json::Value as JsonValue;

use crate::ponos::value::{Value, ValueKey};

pub(crate) fn json_to_value(value: &JsonValue) -> Result<Value, String> {
    match value {
        JsonValue::Null => Ok(Value::Nil),
        JsonValue::Bool(b) => Ok(Value::Boolean(*b)),
        JsonValue::Number(n) => n
            .as_f64()
            .map(Value::Number)
            .ok_or_else(|| "Число вне диапазона f64".to_string()),
        JsonValue::String(s) => Ok(Value::String(s.clone())),
        JsonValue::Array(arr) => {
            let mut items = Vec::with_capacity(arr.len());
            for v in arr.iter() {
                items.push(json_to_value(v)?);
            }
            Ok(Value::Array(Rc::new(RefCell::new(items))))
        }
        JsonValue::Object(obj) => {
            let mut map = HashMap::new();
            for (k, v) in obj.iter() {
                map.insert(ValueKey::String(k.clone()), json_to_value(v)?);
            }
            Ok(Value::Dict(Rc::new(RefCell::new(map))))
        }
    }
}

pub(crate) fn value_to_json(value: &Value) -> Result<JsonValue, String> {
    match value {
        Value::Nil => Ok(JsonValue::Null),
        Value::Boolean(b) => Ok(JsonValue::Bool(*b)),
        Value::Number(n) => serde_json::Number::from_f64(*n)
            .map(JsonValue::Number)
            .ok_or_else(|| "Число не может быть представлено в JSON".to_string()),
        Value::String(s) => Ok(JsonValue::String(s.clone())),
        Value::Array(arr) => {
            let borrowed = arr.borrow();
            let mut items = Vec::with_capacity(borrowed.len());
            for v in borrowed.iter() {
                items.push(value_to_json(v)?);
            }
            Ok(JsonValue::Array(items))
        }
        Value::Dict(dict) => {
            let borrowed = dict.borrow();
            let mut obj = serde_json::Map::new();
            for (k, v) in borrowed.iter() {
                let key = match k {
                    ValueKey::String(s) => s.clone(),
                    ValueKey::Number(n) => n.to_string(),
                    ValueKey::Boolean(b) => b.to_string(),
                };
                obj.insert(key, value_to_json(v)?);
            }
            Ok(JsonValue::Object(obj))
        }
        _ => Err("Невозможно сериализовать этот тип в JSON".to_string()),
    }
}

/// json.десериализовать(строка) -> Value
pub fn json_parse(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("десериализовать требует 1 аргумент: строку с JSON".to_string());
    }

    let source = match &args[0] {
        Value::String(s) => s,
        _ => return Err("Аргумент должен быть строкой".to_string()),
    };

    let parsed: JsonValue =
        serde_json::from_str(source).map_err(|e| format!("Ошибка разбора JSON: {}", e))?;

    json_to_value(&parsed)
}

/// json.сериализовать(значение) -> строка
pub fn json_stringify(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("сериализовать требует 1 аргумент: значение".to_string());
    }

    let json_value = value_to_json(&args[0])?;
    let serialized = serde_json::to_string(&json_value)
        .map_err(|e| format!("Ошибка сериализации JSON: {}", e))?;
    Ok(Value::String(serialized))
}
