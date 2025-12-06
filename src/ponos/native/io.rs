use crate::ponos::value::Value;

pub fn io_print(args: &[Value]) -> Result<Value, String> {
    for arg in args {
        match arg {
            Value::Number(n) => print!("{}", n),
            Value::String(s) => print!("{}", s),
            Value::Boolean(b) => print!("{}", b),
            Value::Nil => print!("ничто"),
            _ => print!("<объект>"),
        }
        print!(" ");
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
