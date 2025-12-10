use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashSet;
use crate::ponos::stdlib;

/// Загрузчик модулей
/// Отвечает за разрешение путей модулей и предотвращение циклических зависимостей
pub struct ModuleLoader {
    /// Путь к директории stdlib (стандартная библиотека)
    stdlib_path: Option<PathBuf>,

    /// Текущий рабочий каталог для разрешения относительных путей
    current_dir: PathBuf,

    /// Стек загружаемых модулей для обнаружения циклов
    loading_stack: Vec<PathBuf>,

    /// Множество уже разрешённых путей (для кэширования)
    resolved_cache: HashSet<PathBuf>,
}

impl ModuleLoader {
    /// Создать новый загрузчик модулей
    pub fn new() -> Self {
        ModuleLoader {
            stdlib_path: None,
            current_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            loading_stack: Vec::new(),
            resolved_cache: HashSet::new(),
        }
    }

    /// Создать загрузчик с указанным путём к stdlib
    pub fn with_stdlib(stdlib_path: PathBuf) -> Self {
        ModuleLoader {
            stdlib_path: Some(stdlib_path),
            current_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            loading_stack: Vec::new(),
            resolved_cache: HashSet::new(),
        }
    }

    /// Установить текущий рабочий каталог
    pub fn set_current_dir(&mut self, dir: PathBuf) {
        self.current_dir = dir;
    }

    /// Разрешить путь к модулю
    ///
    /// Поддерживает:
    /// - Встроенные stdlib модули (приоритет 1)
    /// - Относительные пути: `./модуль`, `../модуль`
    /// - Абсолютные пути: `/путь/к/модулю`
    /// - Stdlib пути: `стандарт/мат` (если stdlib_path установлен)
    pub fn resolve_path(&self, module_path: &str, from_file: Option<&Path>) -> Result<PathBuf, String> {
        // Нормализуем путь (удаляем .pns если есть)
        let normalized = if module_path.ends_with(".pns") {
            &module_path[..module_path.len() - 4]
        } else {
            module_path
        };

        // 0. НОВОЕ: Проверяем встроенные stdlib модули (высший приоритет)
        if stdlib::is_embedded_stdlib(normalized) {
            // Возвращаем виртуальный путь для embedded модулей
            return Ok(PathBuf::from(format!("{}.pns", normalized)));
        }

        // 1. Проверяем абсолютный путь
        if module_path.starts_with('/') {
            return self.resolve_absolute_path(module_path);
        }

        // 2. Проверяем относительный путь
        if module_path.starts_with("./") || module_path.starts_with("../") {
            return self.resolve_relative_path(module_path, from_file);
        }

        // 3. Проверяем stdlib путь
        if let Some(stdlib) = &self.stdlib_path {
            if let Ok(path) = self.resolve_stdlib_path(module_path, stdlib) {
                return Ok(path);
            }
        }

        // 4. По умолчанию - относительно текущего каталога
        self.resolve_default_path(module_path, from_file)
    }

    /// Разрешить абсолютный путь
    fn resolve_absolute_path(&self, module_path: &str) -> Result<PathBuf, String> {
        let path = PathBuf::from(module_path);
        let pns_path = if path.extension().is_some() {
            path
        } else {
            path.with_extension("pns")
        };

        if pns_path.exists() {
            Ok(pns_path)
        } else {
            Err(format!("Модуль не найден по абсолютному пути: {}", module_path))
        }
    }

    /// Разрешить относительный путь
    fn resolve_relative_path(&self, module_path: &str, from_file: Option<&Path>) -> Result<PathBuf, String> {
        let base_dir = if let Some(file) = from_file {
            file.parent()
                .ok_or_else(|| format!("Не удалось получить родительский каталог для {}", file.display()))?
                .to_path_buf()
        } else {
            self.current_dir.clone()
        };

        let path = base_dir.join(module_path);
        let pns_path = if path.extension().is_some() {
            path
        } else {
            path.with_extension("pns")
        };

        if pns_path.exists() {
            Ok(pns_path.canonicalize()
                .map_err(|e| format!("Не удалось канонизировать путь {}: {}", pns_path.display(), e))?)
        } else {
            Err(format!("Модуль не найден по относительному пути: {}", module_path))
        }
    }

    /// Разрешить путь к stdlib
    fn resolve_stdlib_path(&self, module_path: &str, stdlib: &Path) -> Result<PathBuf, String> {
        let path = stdlib.join(module_path);
        let pns_path = if path.extension().is_some() {
            path
        } else {
            path.with_extension("pns")
        };

        if pns_path.exists() {
            Ok(pns_path)
        } else {
            Err(format!("Модуль не найден в stdlib: {}", module_path))
        }
    }

    /// Разрешить путь по умолчанию (относительно from_file или current_dir)
    fn resolve_default_path(&self, module_path: &str, from_file: Option<&Path>) -> Result<PathBuf, String> {
        let base_dir = if let Some(file) = from_file {
            file.parent()
                .ok_or_else(|| format!("Не удалось получить родительский каталог для {}", file.display()))?
                .to_path_buf()
        } else {
            self.current_dir.clone()
        };

        let path = base_dir.join(module_path);
        let pns_path = if path.extension().is_some() {
            path
        } else {
            path.with_extension("pns")
        };

        if pns_path.exists() {
            Ok(pns_path.canonicalize()
                .map_err(|e| format!("Не удалось канонизировать путь {}: {}", pns_path.display(), e))?)
        } else {
            Err(format!("Модуль не найден: {}", module_path))
        }
    }

    /// Начать загрузку модуля (добавить в стек)
    pub fn begin_loading(&mut self, path: &Path) -> Result<(), String> {
        // Для embedded модулей используем путь как есть, для остальных - канонизируем
        let canonical_path = if let Some(module_path) = Self::extract_stdlib_module_path(path) {
            if stdlib::is_embedded_stdlib(&module_path) {
                // Embedded модуль - используем путь как есть
                path.to_path_buf()
            } else {
                // Обычный модуль - канонизируем
                path.canonicalize()
                    .map_err(|e| format!("Не удалось канонизировать путь {}: {}", path.display(), e))?
            }
        } else {
            // Обычный модуль - канонизируем
            path.canonicalize()
                .map_err(|e| format!("Не удалось канонизировать путь {}: {}", path.display(), e))?
        };

        // Проверяем циклическую зависимость
        if self.loading_stack.contains(&canonical_path) {
            let cycle = self.format_cycle(&canonical_path);
            return Err(format!("Обнаружена циклическая зависимость модулей:\n{}", cycle));
        }

        self.loading_stack.push(canonical_path);
        Ok(())
    }

    /// Завершить загрузку модуля (удалить из стека)
    pub fn end_loading(&mut self, path: &Path) {
        // Для embedded модулей используем путь как есть
        let path_to_use = if let Some(module_path) = Self::extract_stdlib_module_path(path) {
            if stdlib::is_embedded_stdlib(&module_path) {
                path.to_path_buf()
            } else {
                path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
            }
        } else {
            path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
        };

        if let Some(pos) = self.loading_stack.iter().position(|p| p == &path_to_use) {
            self.loading_stack.remove(pos);
        }
        self.resolved_cache.insert(path_to_use);
    }

    /// Проверить, загружается ли модуль в данный момент
    pub fn is_loading(&self, path: &Path) -> bool {
        // Для embedded модулей используем путь как есть
        let path_to_use = if let Some(module_path) = Self::extract_stdlib_module_path(path) {
            if stdlib::is_embedded_stdlib(&module_path) {
                path.to_path_buf()
            } else {
                path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
            }
        } else {
            path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
        };

        self.loading_stack.contains(&path_to_use)
    }

    /// Проверить, был ли модуль уже загружен
    pub fn is_loaded(&self, path: &Path) -> bool {
        // Для embedded модулей используем путь как есть
        let path_to_use = if let Some(module_path) = Self::extract_stdlib_module_path(path) {
            if stdlib::is_embedded_stdlib(&module_path) {
                path.to_path_buf()
            } else {
                path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
            }
        } else {
            path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
        };

        self.resolved_cache.contains(&path_to_use)
    }

    /// Форматировать цикл зависимостей для вывода ошибки
    fn format_cycle(&self, new_path: &Path) -> String {
        let mut cycle_str = String::new();
        for (i, path) in self.loading_stack.iter().enumerate() {
            cycle_str.push_str(&format!("  {}. {}\n", i + 1, path.display()));
        }
        cycle_str.push_str(&format!("  {}. {} (цикл!)\n", self.loading_stack.len() + 1, new_path.display()));
        cycle_str
    }

    /// Прочитать содержимое файла модуля
    pub fn read_module_file(&self, path: &Path) -> Result<String, String> {
        // Попытаться извлечь путь модуля из полного пути
        if let Some(module_path) = Self::extract_stdlib_module_path(path) {
            // Проверить встроенные модули
            if let Some(source) = stdlib::get_embedded_source(&module_path) {
                return Ok(source.to_string());
            }
        }

        // Иначе читать из файловой системы
        fs::read_to_string(path)
            .map_err(|e| format!("Не удалось прочитать файл {}: {}", path.display(), e))
    }

    /// Извлекает путь модуля из полного пути файла
    /// Например: "/some/path/стд/математика.pns" -> "стд/математика"
    fn extract_stdlib_module_path(full_path: &Path) -> Option<String> {
        let path_str = full_path.to_str()?;

        // Ищем "стд/" в пути
        if let Some(idx) = path_str.find("стд/") {
            let module_path = &path_str[idx..];
            // Убираем расширение .pns
            if let Some(without_ext) = module_path.strip_suffix(".pns") {
                return Some(without_ext.to_string());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    #[test]
    fn test_module_loader_creation() {
        let loader = ModuleLoader::new();
        assert!(loader.stdlib_path.is_none());
    }

    #[test]
    fn test_module_loader_with_stdlib() {
        let stdlib = PathBuf::from("/usr/lib/ponos");
        let loader = ModuleLoader::with_stdlib(stdlib.clone());
        assert_eq!(loader.stdlib_path, Some(stdlib));
    }

    #[test]
    fn test_resolve_absolute_path() {
        let loader = ModuleLoader::new();

        // Создаём временный файл для теста
        let temp_path = env::temp_dir().join("test_module.pns");
        fs::write(&temp_path, "// test").unwrap();

        let result = loader.resolve_path(temp_path.to_str().unwrap(), None);
        assert!(result.is_ok());

        // Очистка
        fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_resolve_relative_path() {
        let loader = ModuleLoader::new();

        // Создаём временную структуру
        let temp_dir = env::temp_dir().join("ponos_test");
        fs::create_dir_all(&temp_dir).ok();

        let module_path = temp_dir.join("module.pns");
        fs::write(&module_path, "// test").unwrap();

        let from_file = temp_dir.join("main.pns");

        let result = loader.resolve_path("./module", Some(&from_file));
        assert!(result.is_ok());

        // Очистка
        fs::remove_file(module_path).ok();
        fs::remove_dir(temp_dir).ok();
    }

    #[test]
    fn test_cycle_detection() {
        let mut loader = ModuleLoader::new();

        // Создаём временный файл
        let temp_path = env::temp_dir().join("cycle_test.pns");
        fs::write(&temp_path, "// test").unwrap();

        // Начинаем загрузку
        assert!(loader.begin_loading(&temp_path).is_ok());

        // Пытаемся загрузить тот же модуль снова - должен обнаружить цикл
        let result = loader.begin_loading(&temp_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Обнаружена циклическая зависимость"));

        // Очистка
        fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_is_loading() {
        let mut loader = ModuleLoader::new();

        let temp_path = env::temp_dir().join("loading_test.pns");
        fs::write(&temp_path, "// test").unwrap();

        assert!(!loader.is_loading(&temp_path));
        loader.begin_loading(&temp_path).unwrap();
        assert!(loader.is_loading(&temp_path));
        loader.end_loading(&temp_path);
        assert!(!loader.is_loading(&temp_path));

        // Очистка
        fs::remove_file(temp_path).ok();
    }

    #[test]
    fn test_is_loaded() {
        let mut loader = ModuleLoader::new();

        let temp_path = env::temp_dir().join("loaded_test.pns");
        fs::write(&temp_path, "// test").unwrap();

        assert!(!loader.is_loaded(&temp_path));
        loader.begin_loading(&temp_path).unwrap();
        loader.end_loading(&temp_path);
        assert!(loader.is_loaded(&temp_path));

        // Очистка
        fs::remove_file(temp_path).ok();
    }
}
