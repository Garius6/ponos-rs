use crate::ponos::native::json::{json_to_value, value_to_json};
use crate::ponos::value::{Value, ValueKey};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;
use ureq::{AgentBuilder, Error as UreqError};

struct RequestOptions {
    headers: HashMap<String, String>,
    body: Option<String>,
    json_body: Option<String>,
    timeout: Option<Duration>,
    expect_json: bool,
}

pub fn http_request(args: &[Value]) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err("запрос требует 2 или 3 аргумента: метод, url, [опции]".to_string());
    }

    let method = match &args[0] {
        Value::String(s) => s.to_uppercase(),
        _ => return Err("Первый аргумент (метод) должен быть строкой".to_string()),
    };

    let url = match &args[1] {
        Value::String(s) => s.clone(),
        _ => return Err("Второй аргумент (url) должен быть строкой".to_string()),
    };

    let mut options = RequestOptions {
        headers: HashMap::new(),
        body: None,
        json_body: None,
        timeout: None,
        expect_json: false,
    };

    if args.len() == 3 {
        options = parse_options(&args[2], false)?;
    }

    perform_request(&method, &url, options)
}

pub fn http_get(args: &[Value]) -> Result<Value, String> {
    if args.len() < 1 || args.len() > 2 {
        return Err("получить требует 1 или 2 аргумента: url, [опции]".to_string());
    }

    let url = match &args[0] {
        Value::String(s) => s.clone(),
        _ => return Err("Первый аргумент (url) должен быть строкой".to_string()),
    };

    let mut options = RequestOptions {
        headers: HashMap::new(),
        body: None,
        json_body: None,
        timeout: None,
        expect_json: false,
    };

    if args.len() == 2 {
        options = parse_options(&args[1], false)?;
    }

    perform_request("GET", &url, options)
}

pub fn http_request_json(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 || args.len() > 4 {
        return Err(
            "запрос_json требует 3 или 4 аргумента: метод, url, данные, [опции]".to_string(),
        );
    }

    let method = match &args[0] {
        Value::String(s) => s.to_uppercase(),
        _ => return Err("Первый аргумент (метод) должен быть строкой".to_string()),
    };

    let url = match &args[1] {
        Value::String(s) => s.clone(),
        _ => return Err("Второй аргумент (url) должен быть строкой".to_string()),
    };

    let json_body = serde_json::to_string(&value_to_json(&args[2])?)
        .map_err(|e| format!("Ошибка сериализации JSON тела: {}", e))?;

    let mut options = RequestOptions {
        headers: HashMap::new(),
        body: None,
        json_body: Some(json_body),
        timeout: None,
        expect_json: true,
    };

    if args.len() == 4 {
        let mut parsed = parse_options(&args[3], true)?;
        if parsed.json_body.is_some() {
            return Err(
                "В запрос_json тело задается третьим аргументом, уберите 'json' из опций"
                    .to_string(),
            );
        }
        parsed.json_body = options.json_body.clone();
        options = parsed;
    }

    perform_request(&method, &url, options)
}

fn parse_options(value: &Value, expect_json_default: bool) -> Result<RequestOptions, String> {
    let dict = match value {
        Value::Dict(d) => d,
        _ => return Err("Опции должны быть словарем".to_string()),
    };

    let mut options = RequestOptions {
        headers: HashMap::new(),
        body: None,
        json_body: None,
        timeout: None,
        expect_json: expect_json_default,
    };

    for (key, val) in dict.borrow().iter() {
        let key_str = match key {
            ValueKey::String(s) => s,
            _ => return Err("Ключи опций должны быть строками".to_string()),
        };

        match key_str.as_str() {
            "заголовки" => {
                options.headers = parse_headers(val)?;
            }
            "тело" => {
                let body = match val {
                    Value::String(s) => s.clone(),
                    _ => return Err("Опция 'тело' должна быть строкой".to_string()),
                };
                options.body = Some(body);
            }
            "json" => {
                options.json_body = Some(
                    serde_json::to_string(&value_to_json(val)?)
                        .map_err(|e| format!("Ошибка сериализации JSON тела: {}", e))?,
                );
            }
            "таймаут_мс" => {
                let millis = match val {
                    Value::Number(n) if *n >= 0.0 => *n as u64,
                    _ => {
                        return Err(
                            "Опция 'таймаут_мс' должна быть неотрицательным числом".to_string()
                        );
                    }
                };
                options.timeout = Some(Duration::from_millis(millis));
            }
            "ожидать_json" => {
                let flag = match val {
                    Value::Boolean(b) => *b,
                    _ => return Err("Опция 'ожидать_json' должна быть булевой".to_string()),
                };
                options.expect_json = flag;
            }
            unknown => {
                return Err(format!("Неизвестная опция '{}'", unknown));
            }
        }
    }

    if options.body.is_some() && options.json_body.is_some() {
        return Err("Нельзя указывать одновременно 'тело' и 'json'".to_string());
    }

    Ok(options)
}

fn parse_headers(value: &Value) -> Result<HashMap<String, String>, String> {
    let dict = match value {
        Value::Dict(d) => d,
        _ => return Err("Заголовки должны быть словарем".to_string()),
    };

    let mut headers = HashMap::new();
    for (key, val) in dict.borrow().iter() {
        let name = match key {
            ValueKey::String(s) => s.clone(),
            _ => return Err("Имена заголовков должны быть строками".to_string()),
        };

        let value_str = match val {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Boolean(b) => b.to_string(),
            _ => {
                return Err("Значения заголовков должны быть строками/числами/булевыми".to_string());
            }
        };

        headers.insert(name, value_str);
    }

    Ok(headers)
}

fn perform_request(method: &str, url: &str, options: RequestOptions) -> Result<Value, String> {
    let mut builder = AgentBuilder::new();
    if let Some(timeout) = options.timeout {
        builder = builder.timeout(timeout);
    }
    let agent = builder.build();

    let mut request = agent.request(method, url);
    for (name, value) in options.headers.iter() {
        request = request.set(name, value);
    }

    let response_result = if let Some(json_body) = options.json_body {
        request
            .set("Content-Type", "application/json")
            .send_string(&json_body)
    } else if let Some(body) = options.body {
        request.send_string(&body)
    } else {
        request.call()
    };

    let response = match response_result {
        Ok(res) => res,
        Err(UreqError::Status(_, res)) => res,
        Err(UreqError::Transport(err)) => {
            return Err(format!("Ошибка HTTP-запроса: {}", err));
        }
    };

    let status = response.status();
    let header_map = collect_headers(&response);
    let body = response
        .into_string()
        .map_err(|e| format!("Ошибка чтения тела ответа: {}", e))?;

    let should_parse_json = options.expect_json || has_json_content_type(&header_map);
    let parsed_json = if should_parse_json {
        if body.trim().is_empty() {
            Some(Value::Nil)
        } else {
            match serde_json::from_str::<serde_json::Value>(&body) {
                Ok(json) => Some(json_to_value(&json)?),
                Err(err) => {
                    if options.expect_json {
                        return Err(format!("Ошибка разбора JSON ответа: {}", err));
                    }
                    None
                }
            }
        }
    } else {
        None
    };

    let mut result = HashMap::new();
    result.insert(
        ValueKey::String("статус".to_string()),
        Value::Number(status as f64),
    );
    result.insert(
        ValueKey::String("заголовки".to_string()),
        headers_to_value(header_map),
    );
    result.insert(ValueKey::String("тело".to_string()), Value::String(body));
    result.insert(
        ValueKey::String("json".to_string()),
        parsed_json.unwrap_or(Value::Nil),
    );

    Ok(Value::Dict(Rc::new(RefCell::new(result))))
}

fn collect_headers(response: &ureq::Response) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for name in response.headers_names() {
        let combined = response.all(&name).join(", ");
        headers.insert(name, combined);
    }
    headers
}

fn has_json_content_type(headers: &HashMap<String, String>) -> bool {
    headers.iter().any(|(name, value)| {
        name.eq_ignore_ascii_case("content-type")
            && value.to_ascii_lowercase().contains("application/json")
    })
}

fn headers_to_value(headers: HashMap<String, String>) -> Value {
    let mut map = HashMap::new();
    for (k, v) in headers.into_iter() {
        map.insert(ValueKey::String(k), Value::String(v));
    }
    Value::Dict(Rc::new(RefCell::new(map)))
}
