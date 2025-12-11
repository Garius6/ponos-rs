use ponos_rs::ponos::Ponos;
use std::path::PathBuf;

/// Вспомогательная функция для запуска .pns файла
fn run_pns_file(filename: &str) -> Result<(), String> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures");
    path.push(filename);

    let source = std::fs::read_to_string(&path)
        .map_err(|e| format!("Не удалось прочитать файл {:?}: {}", path, e))?;

    let mut ponos = Ponos::new();

    // Перехватываем панику, если она случится
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ponos.run_source(source);
    }));

    match result {
        Ok(_) => Ok(()),
        Err(e) => {
            if let Some(s) = e.downcast_ref::<String>() {
                Err(format!("Паника при выполнении {}: {}", filename, s))
            } else if let Some(s) = e.downcast_ref::<&str>() {
                Err(format!("Паника при выполнении {}: {}", filename, s))
            } else {
                Err(format!("Неизвестная паника при выполнении {}", filename))
            }
        }
    }
}

#[test]
fn test_simple_class() {
    run_pns_file("test_simple_class.pns")
        .expect("test_simple_class.pns должен выполниться без ошибок");
}

#[test]
fn test_class_constructor_param() {
    run_pns_file("test_class_constructor_param.pns")
        .expect("test_class_constructor_param.pns должен выполниться без ошибок");
}

#[test]
fn test_phase1_complete() {
    run_pns_file("test_phase1_complete.pns")
        .expect("test_phase1_complete.pns должен выполниться без ошибок");
}

#[test]
fn test_exception_basic() {
    run_pns_file("test_exception_basic.pns")
        .expect("Базовый try/catch должен выполняться без ошибок");
}

#[test]
fn test_exception_no_var() {
    run_pns_file("test_exception_no_var.pns")
        .expect("Перехват без переменной должен выполняться без ошибок");
}

#[test]
fn test_exception_nested() {
    run_pns_file("test_exception_nested.pns")
        .expect("Вложенные try/catch должны выполняться без ошибок");
}

#[test]
fn test_exception_function() {
    run_pns_file("test_exception_function.pns")
        .expect("Исключения из функций должны перехватываться");
}

#[test]
fn test_exception_dynamic() {
    run_pns_file("test_exception_dynamic.pns")
        .expect("Динамическое сообщение исключения должно работать");
}

#[test]
fn test_exception_unhandled_panics() {
    assert!(
        run_pns_file("test_exception_unhandled.pns").is_err(),
        "Необработанное исключение должно приводить к ошибке"
    );
}

#[test]
fn test_json_module() {
    run_pns_file("test_json_module.pns")
        .expect("Нативный модуль JSON должен корректно разбирать и сериализовывать данные");
}
