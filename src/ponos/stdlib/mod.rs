use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Встроенные модули стандартной библиотеки
pub static EMBEDDED_STDLIB: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut map = HashMap::new();

    // Встраиваем исходники .pns файлов на этапе компиляции
    map.insert("стд/математика", include_str!("../../../stdlib/математика.pns"));
    
    map
});

/// Проверяет, является ли путь встроенным stdlib модулем
pub fn is_embedded_stdlib(path: &str) -> bool {
    EMBEDDED_STDLIB.contains_key(path)
}

/// Получает исходный код встроенного модуля
pub fn get_embedded_source(path: &str) -> Option<&'static str> {
    EMBEDDED_STDLIB.get(path).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_stdlib_exists() {
        assert!(is_embedded_stdlib("стд/математика"));
        assert!(!is_embedded_stdlib("не_существует"));
    }

    #[test]
    fn test_get_embedded_source() {
        let math_source = get_embedded_source("стд/математика");
        assert!(math_source.is_some());
        assert!(math_source.unwrap().contains("экспорт пер ПИ"));
    }
}
