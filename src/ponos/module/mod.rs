mod loader;
mod resolver;

pub use loader::ModuleLoader;
pub use resolver::{ModuleResolver, LoadedModule};

use std::collections::HashMap;
use std::path::PathBuf;
use crate::ponos::ast::{Program, Statement, ModuleBlock};
use crate::ponos::span::Span;
use crate::ponos::symbol_table::ScopeId;

/// Информация об импорте
#[derive(Debug, Clone)]
pub struct Import {
    /// Путь к модулю
    pub module_path: String,
    /// Псевдоним для пространства имен (опционально)
    pub alias: Option<String>,
    /// Позиция в исходном коде
    pub span: Span,
}

impl Import {
    /// Создать новый импорт
    pub fn new(module_path: String, alias: Option<String>, span: Span) -> Self {
        Import {
            module_path,
            alias,
            span,
        }
    }
}

/// Информация о модуле
/// Метаданные модуля + ссылка на его область видимости в SymbolTable
#[derive(Debug, Clone)]
pub struct Module {
    /// Имя модуля (из объявления `модуль имя;`)
    pub name: String,

    /// Путь к файлу модуля
    pub file_path: PathBuf,

    /// AST модуля
    pub ast: Program,

    /// Идентификатор области видимости модуля в SymbolTable
    pub scope_id: ScopeId,

    /// Список импортов в этом модуле
    pub imports: Vec<Import>,
}

impl Module {
    /// Создать новый модуль
    pub fn new(name: String, file_path: PathBuf, ast: Program, scope_id: ScopeId) -> Self {
        Module {
            name,
            file_path,
            ast,
            scope_id,
            imports: Vec::new(),
        }
    }

    /// Добавить импорт
    pub fn add_import(&mut self, import: Import) {
        self.imports.push(import);
    }

    /// Получить количество импортов
    pub fn import_count(&self) -> usize {
        self.imports.len()
    }
}

/// Реестр модулей
/// Управляет загруженными модулями
#[derive(Debug)]
pub struct ModuleRegistry {
    /// Загруженные модули (имя -> модуль)
    modules: HashMap<String, Module>,
}

impl ModuleRegistry {
    /// Создать новый реестр модулей
    pub fn new() -> Self {
        ModuleRegistry {
            modules: HashMap::new(),
        }
    }

    /// Зарегистрировать модуль
    pub fn register(&mut self, module: Module) {
        self.modules.insert(module.name.clone(), module);
    }

    /// Получить модуль по имени
    pub fn get(&self, name: &str) -> Option<&Module> {
        self.modules.get(name)
    }

    /// Получить мутабельный модуль по имени
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Module> {
        self.modules.get_mut(name)
    }

    /// Проверить, зарегистрирован ли модуль
    pub fn contains(&self, name: &str) -> bool {
        self.modules.contains_key(name)
    }

    /// Получить количество загруженных модулей
    pub fn count(&self) -> usize {
        self.modules.len()
    }

    /// Получить все имена модулей
    pub fn module_names(&self) -> Vec<String> {
        self.modules.keys().cloned().collect()
    }
}

/// Присоединить AST модуля к основному AST
///
/// Оборачивает statements модуля в ModuleBlock и добавляет в основной AST
///
/// # Параметры
/// - `main_ast`: Основной AST программы
/// - `loaded_module`: Загруженный модуль с AST и пространством имен
pub fn merge_module_ast(main_ast: &mut Program, loaded_module: LoadedModule) {
    // Создаем ModuleBlock для модуля
    let module_block = ModuleBlock {
        namespace: loaded_module.namespace,
        statements: loaded_module.ast.statements,
        span: Span::default(),
    };

    // Добавляем ModuleBlock в основной AST
    main_ast.statements.push(Statement::ModuleBlock(module_block));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ponos::ast::Program;
    use crate::ponos::symbol_table::{SymbolTable, SymbolKind, Symbol};

    #[test]
    fn test_import_creation() {
        let import = Import::new(
            "стандарт/мат".to_string(),
            None,
            Span::new(0, 20),
        );
        assert_eq!(import.module_path, "стандарт/мат");
        assert!(import.alias.is_none());
    }

    #[test]
    fn test_module_creation() {
        let program = Program { statements: Vec::new() };
        let module = Module::new(
            "test_module".to_string(),
            PathBuf::from("/path/to/test.pns"),
            program,
            ScopeId(1),
        );
        assert_eq!(module.name, "test_module");
        assert_eq!(module.import_count(), 0);
    }

    #[test]
    fn test_module_add_import() {
        let program = Program { statements: Vec::new() };
        let mut module = Module::new(
            "test_module".to_string(),
            PathBuf::from("/path/to/test.pns"),
            program,
            ScopeId(1),
        );

        let import = Import::new(
            "стандарт/мат".to_string(),
            None,
            Span::new(0, 20),
        );
        module.add_import(import);

        assert_eq!(module.import_count(), 1);
        assert_eq!(module.imports[0].module_path, "стандарт/мат");
    }

    #[test]
    fn test_module_registry_creation() {
        let registry = ModuleRegistry::new();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_module_registry_register() {
        let mut registry = ModuleRegistry::new();

        let program = Program { statements: Vec::new() };
        let module = Module::new(
            "test_module".to_string(),
            PathBuf::from("/path/to/test.pns"),
            program,
            ScopeId(1),
        );

        registry.register(module);

        assert_eq!(registry.count(), 1);
        assert!(registry.contains("test_module"));
        assert!(!registry.contains("nonexistent"));
    }

    #[test]
    fn test_module_registry_get() {
        let mut registry = ModuleRegistry::new();

        let program = Program { statements: Vec::new() };
        let module = Module::new(
            "test_module".to_string(),
            PathBuf::from("/path/to/test.pns"),
            program,
            ScopeId(1),
        );

        registry.register(module);

        let retrieved = registry.get("test_module");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test_module");

        let not_found = registry.get("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_module_with_symbol_table() {
        // Интеграционный тест: модуль + таблица символов
        let mut symbol_table = SymbolTable::new();

        // Создаём область для модуля
        let scope_id = symbol_table.push_scope();

        // Добавляем символы в scope модуля
        symbol_table.define_in_scope(scope_id, Symbol::new(
            "exported_func".to_string(),
            SymbolKind::Function,
            true,
            Span::new(0, 10),
        )).unwrap();

        symbol_table.define_in_scope(scope_id, Symbol::new(
            "private_var".to_string(),
            SymbolKind::Variable,
            false,
            Span::new(10, 20),
        )).unwrap();

        // Создаём модуль с этой областью
        let program = Program { statements: Vec::new() };
        let module = Module::new(
            "test_module".to_string(),
            PathBuf::from("/path/to/test.pns"),
            program,
            scope_id,
        );

        // Проверяем, что можем получить экспорты модуля через таблицу символов
        let scope = symbol_table.get_scope(module.scope_id);
        let exports = scope.exported_symbols();

        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].name, "exported_func");
    }
}
