use crate::ponos::value::Value;
use std::process::Command;

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
