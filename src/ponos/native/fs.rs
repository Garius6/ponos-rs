use crate::ponos::value::Value;
use std::fs;

pub fn fs_read(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("читать ожидает 1 аргумент (путь)".to_string());
    }

    let path = match &args[0] {
        Value::String(s) => s,
        _ => return Err("Путь должен быть строкой".to_string()),
    };

    let content = fs::read_to_string(path).map_err(|e| format!("Ошибка чтения: {}", e))?;

    Ok(Value::String(content))
}

pub fn fs_write(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("писать ожидает 2 аргумента".to_string());
    }

    let path = match &args[0] {
        Value::String(s) => s,
        _ => return Err("Путь должен быть строкой".to_string()),
    };

    let content = match &args[1] {
        Value::String(s) => s,
        _ => return Err("Содержимое должно быть строкой".to_string()),
    };

    fs::write(path, content).map_err(|e| format!("Ошибка записи: {}", e))?;

    Ok(Value::Nil)
}

pub fn fs_exists(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("существует ожидает 1 аргумент".to_string());
    }

    let path = match &args[0] {
        Value::String(s) => s,
        _ => return Err("Путь должен быть строкой".to_string()),
    };

    Ok(Value::Boolean(std::path::Path::new(path).exists()))
}

pub fn fs_delete(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("удалить ожидает 1 аргумент".to_string());
    }

    let path = match &args[0] {
        Value::String(s) => s,
        _ => return Err("Путь должен быть строкой".to_string()),
    };

    fs::remove_file(path).map_err(|e| format!("Ошибка удаления: {}", e))?;

    Ok(Value::Nil)
}
