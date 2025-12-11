use crate::ponos::value::Value;
use std::cell::RefCell;
use std::env;
use std::process::Command;
use std::rc::Rc;

thread_local! {
    static CLI_ARGS: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

pub fn sys_execute(args: &[Value]) -> Result<Value, String> {
    if args.is_empty() {
        return Err("выполнить ожидает команду".to_string());
    }

    let command = match &args[0] {
        Value::String(s) => s,
        _ => return Err("Команда должна быть строкой".to_string()),
    };

    let cmd_args: Vec<String> = args[1..]
        .iter()
        .map(|v| match v {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Boolean(b) => b.to_string(),
            _ => String::new(),
        })
        .collect();

    let output = Command::new(command)
        .args(&cmd_args)
        .output()
        .map_err(|e| format!("Ошибка выполнения: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(Value::String(stdout))
}

pub fn env_get(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("получить_переменную требует 1 аргумент: имя переменной".to_string());
    }

    let var_name = match &args[0] {
        Value::String(s) => s,
        _ => return Err("Имя переменной должно быть строкой".to_string()),
    };

    match env::var(var_name) {
        Ok(value) => Ok(Value::String(value)),
        Err(_) => Ok(Value::Nil), // Переменная не найдена
    }
}

/// система.задать_переменную(имя, значение) -> Nil
pub fn env_set(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("задать_переменную требует 2 аргумента: имя и значение".to_string());
    }

    let var_name = match &args[0] {
        Value::String(s) => s,
        _ => return Err("Имя переменной должно быть строкой".to_string()),
    };

    let var_value = match &args[1] {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Boolean(b) => b.to_string(),
        _ => return Err("Значение должно быть строкой, числом или булевым".to_string()),
    };

    unsafe {
        env::set_var(var_name, var_value);
    }
    Ok(Value::Nil)
}

/// Устанавливает аргументы командной строки (вызывается из main.rs)
pub fn set_cli_args(args: Vec<String>) {
    CLI_ARGS.with(|a| {
        *a.borrow_mut() = args;
    });
}

/// система.аргументы() -> Array
pub fn get_args(_args: &[Value]) -> Result<Value, String> {
    let args = CLI_ARGS.with(|a| {
        a.borrow()
            .iter()
            .map(|s| Value::String(s.clone()))
            .collect::<Vec<Value>>()
    });

    Ok(Value::Array(Rc::new(RefCell::new(args))))
}
