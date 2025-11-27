#[derive(Clone, Debug)]
pub enum Value {
    Number(f64),
    String(String),
    Boolean(bool),
}

pub fn is_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Boolean(x), Value::Boolean(y)) => x == y,
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
