use crate::ponos::value::{Class, Instance, Value};
use std::{
    cell::RefCell,
    collections::HashMap,
    fs::{self, DirEntry},
    rc::Rc,
    vec,
};

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

pub fn fs_read_dir(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("Неверное количество параметров!".to_string());
    }
    let path = match &args[0] {
        Value::String(s) => s.clone(),
        _ => return Err("Параметр должен быть строкой!".to_string()),
    };
    let mut res: Vec<Value> = Vec::new();

    let entries = fs::read_dir(path).expect("Cannot read dir");
    for entry in entries {
        let info = entry.expect("Cannot get info");
        let instance = Instance {
            class: Rc::new(create_file_info_class()),
            fields: HashMap::from([
                (
                    "имя".to_string(),
                    Value::String(info.file_name().into_string().expect("Cannot transform")),
                ),
                (
                    "это_директория".to_string(),
                    Value::Boolean(info.file_type().expect("Cannot transform").is_dir()),
                ),
                (
                    "абсолютный_путь".to_string(),
                    Value::String(info.path().canonicalize().unwrap().display().to_string())
                )
            ]),
        };
        res.push(Value::Instance(Rc::new(RefCell::new(instance))));
    }
    Ok(Value::Array(Rc::new(RefCell::new(res))))
}

fn create_file_info_class() -> Class {
    let class = Class {
        name: "ИнформацияОФайле".to_string(),
        methods: HashMap::new(),
        fields: vec![
            "абсолютный_путь".to_string(),
            "это_директория".to_string(),
            "имя".to_string(),
        ],
        parent: None,
    };

    class
}

// Заготовка под ООП-style API

fn create_file_class() -> Class {
    let class = Class {
        name: "Файл".to_string(),
        methods: HashMap::new(),
        fields: vec![
            "путь".to_string(),
            "это_директория".to_string(),
            "имя".to_string(),
        ],
        parent: None,
    };

    class
}

pub fn file_constructor(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("Неверное количество параметров!".to_string());
    }
    let path = match &args[0] {
        Value::String(s) => s.clone(),
        _ => return Err("Параметр должен быть строкой!".to_string()),
    };

    if !fs::exists(path.clone()).expect("Не удалось проверить существование файла")
    {
        return Err("По данному пути не существует файла".to_string());
    }

    let instance = Instance {
        class: Rc::new(create_file_class()),
        fields: HashMap::from([("путь".to_string(), Value::String(path))]),
    };
    Ok(Value::Instance(Rc::new(RefCell::new(instance))))
}

pub fn file_read_method(instance: &Rc<RefCell<Instance>>, args: &[Value]) -> Result<Value, String> {
    todo!()
}

pub fn file_write_method(
    instance: &Rc<RefCell<Instance>>,
    args: &[Value],
) -> Result<Value, String> {
    todo!()
}

pub fn file_delete_method(
    instance: &Rc<RefCell<Instance>>,
    args: &[Value],
) -> Result<Value, String> {
    todo!()
}
