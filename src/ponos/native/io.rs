use crate::ponos::value::Value;

fn format_value(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Nil => "ничто".to_string(),
        Value::Array(arr) => {
            let items: Vec<String> = arr.borrow().iter().map(format_value).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Dict(dict) => {
            let items: Vec<String> = dict.borrow().iter().map(|(k, v)| {
                let key_str = match k {
                    crate::ponos::value::ValueKey::String(s) => format!("\"{}\"", s),
                    crate::ponos::value::ValueKey::Number(n) => n.to_string(),
                    crate::ponos::value::ValueKey::Boolean(b) => b.to_string(),
                };
                format!("{}: {}", key_str, format_value(v))
            }).collect();
            format!("{{{}}}", items.join(", "))
        }
        _ => "<объект>".to_string(),
    }
}

pub fn io_print(args: &[Value]) -> Result<Value, String> {
    for arg in args {
        print!("{} ", format_value(arg));
    }
    println!();
    Ok(Value::Nil)
}

pub fn io_input(args: &[Value]) -> Result<Value, String> {
    use std::io::{self, Write};

    if !args.is_empty() {
        if let Value::String(prompt) = &args[0] {
            print!("{}", prompt);
            io::stdout().flush().unwrap();
        }
    }

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("Ошибка ввода: {}", e))?;

    Ok(Value::String(input.trim().to_string()))
}
