use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::ponos::ast::{Program, Statement};
use crate::ponos::parser::PonosParser;
use crate::ponos::symbol_table::{SymbolTable, ScopeId, Symbol, SymbolKind};
use crate::ponos::span::Span;
use crate::ponos::native::NativeModuleRegistry;
use super::loader::ModuleLoader;

/// Загруженный модуль с метаданными
#[derive(Debug, Clone)]
pub struct LoadedModule {
    /// Имя пространства имен для доступа к модулю
    pub namespace: String,
    /// AST модуля
    pub ast: Program,
    /// Список экспортированных символов
    pub exports: Vec<String>,
    /// Путь к файлу модуля
    pub file_path: PathBuf,
    /// ID области видимости модуля в SymbolTable
    pub scope_id: ScopeId,
}

/// Разрешитель модулей - загружает модули и преобразует их в LoadedModule
pub struct ModuleResolver {
    /// Загрузчик модулей для разрешения путей и чтения файлов
    loader: ModuleLoader,
    /// Парсер для разбора модулей
    parser: PonosParser,
    /// Кэш загруженных модулей (путь файла -> LoadedModule)
    loaded_modules: HashMap<PathBuf, LoadedModule>,
    /// Реестр нативных модулей
    native_registry: NativeModuleRegistry,
}

impl ModuleResolver {
    /// Создать новый разрешитель модулей
    pub fn new() -> Self {
        ModuleResolver {
            loader: ModuleLoader::new(),
            parser: PonosParser::new(),
            loaded_modules: HashMap::new(),
            native_registry: NativeModuleRegistry::new(),
        }
    }

    /// Создать разрешитель с указанным путём к stdlib
    pub fn with_stdlib(stdlib_path: PathBuf) -> Self {
        ModuleResolver {
            loader: ModuleLoader::with_stdlib(stdlib_path),
            parser: PonosParser::new(),
            loaded_modules: HashMap::new(),
            native_registry: NativeModuleRegistry::new(),
        }
    }

    /// Получить ссылку на реестр нативных модулей
    pub fn native_registry(&self) -> &NativeModuleRegistry {
        &self.native_registry
    }

    /// Загрузить модуль по пути импорта
    ///
    /// # Параметры
    /// - `import_path`: Путь к модулю из оператора импорта (например, "математика", "путь/к/модулю")
    /// - `alias`: Опциональный псевдоним из модификатора "как"
    /// - `from_file`: Опциональный путь к файлу, из которого выполняется импорт (для относительных путей)
    /// - `symbol_table`: Таблица символов для регистрации экспортов модуля
    ///
    /// # Возвращает
    /// `LoadedModule` с AST модуля, пространством имен, списком экспортов и scope_id
    pub fn load_module(
        &mut self,
        import_path: &str,
        alias: Option<String>,
        from_file: Option<&Path>,
        symbol_table: &mut SymbolTable,
    ) -> Result<LoadedModule, String> {
        // 1. Проверяем, является ли это нативным модулем
        if self.native_registry.is_native_module(import_path) {
            return self.load_native_module(import_path, alias, symbol_table);
        }

        // 2. Разрешаем путь к файлу модуля
        let module_path = self.loader.resolve_path(import_path, from_file)?;

        // 3. Проверяем кэш
        if let Some(cached) = self.loaded_modules.get(&module_path) {
            // Если есть псевдоним, создаем новый LoadedModule с обновленным namespace
            if let Some(alias_name) = alias {
                return Ok(LoadedModule {
                    namespace: alias_name,
                    ast: cached.ast.clone(),
                    exports: cached.exports.clone(),
                    file_path: cached.file_path.clone(),
                    scope_id: cached.scope_id,
                });
            }
            return Ok(cached.clone());
        }

        // 4. Начинаем загрузку (проверка циклических зависимостей)
        self.loader.begin_loading(&module_path)?;

        // 5. Читаем содержимое файла
        let source = self.loader.read_module_file(&module_path)?;

        // 6. Парсим модуль
        let ast = self.parser
            .parse(source)
            .map_err(|e| format!("Ошибка парсинга модуля {}: {:?}", module_path.display(), e))?;

        // 7. Извлекаем список экспортов
        let exports = Self::collect_exports(&ast);

        // 8. Определяем имя пространства имен
        let namespace = Self::extract_namespace(import_path, alias.clone());

        // 9. Создаем scope для модуля в SymbolTable
        let scope_id = symbol_table.push_scope();

        // 10. Регистрируем экспортированные символы в SymbolTable
        Self::register_exports_in_symbol_table(&ast, scope_id, symbol_table)?;

        // 11. Создаем LoadedModule
        let loaded_module = LoadedModule {
            namespace,
            ast,
            exports,
            file_path: module_path.clone(),
            scope_id,
        };

        // 12. Завершаем загрузку
        self.loader.end_loading(&module_path);

        // 13. Кэшируем
        self.loaded_modules.insert(module_path, loaded_module.clone());

        Ok(loaded_module)
    }

    /// Загрузить нативный модуль
    fn load_native_module(
        &mut self,
        import_path: &str,
        alias: Option<String>,
        symbol_table: &mut SymbolTable,
    ) -> Result<LoadedModule, String> {
        let native_module = self.native_registry.get_module(import_path)
            .ok_or_else(|| format!("Нативный модуль '{}' не найден", import_path))?;

        // Определяем имя пространства имен
        let namespace = Self::extract_namespace(import_path, alias.clone());

        // Создаем пустой AST (нативные модули не имеют AST)
        let ast = Program {
            statements: Vec::new(),
        };

        // Копируем список экспортов
        let exports = native_module.exports.clone();

        // Создаем scope для модуля в SymbolTable
        let scope_id = symbol_table.push_scope();

        // Регистрируем экспортированные символы как функции
        for export_name in &exports {
            let symbol = Symbol::new(
                export_name.clone(),
                SymbolKind::Function,
                true,
                Span::default(),
            );
            symbol_table
                .define_in_scope(scope_id, symbol)
                .map_err(|e| format!("Ошибка регистрации нативной функции '{}': {}", export_name, e))?;
        }

        // Создаем LoadedModule с фиктивным путём
        let loaded_module = LoadedModule {
            namespace,
            ast,
            exports,
            file_path: PathBuf::from(format!("<native:{}>", import_path)),
            scope_id,
        };

        Ok(loaded_module)
    }

    /// Извлечь имя пространства имен из пути импорта и псевдонима
    ///
    /// # Примеры
    /// - `extract_namespace("математика", None)` → `"математика"`
    /// - `extract_namespace("путь/к/математика", None)` → `"математика"`
    /// - `extract_namespace("математика", Some("мат"))` → `"мат"`
    fn extract_namespace(import_path: &str, alias: Option<String>) -> String {
        // Если есть псевдоним, используем его
        if let Some(alias_name) = alias {
            return alias_name;
        }

        // Иначе берем последний элемент пути и убираем расширение .pns
        let last_part = import_path
            .split('/')
            .last()
            .unwrap_or(import_path);

        // Убираем расширение .pns, если есть
        if let Some(name_without_ext) = last_part.strip_suffix(".pns") {
            name_without_ext.to_string()
        } else {
            last_part.to_string()
        }
    }

    /// Собрать список экспортированных символов из AST модуля
    ///
    /// Проходит по всем statements и собирает имена с флагом `is_exported`
    fn collect_exports(ast: &Program) -> Vec<String> {
        let mut exports = Vec::new();

        for statement in &ast.statements {
            match statement {
                Statement::VarDecl(var_decl) if var_decl.is_exported => {
                    exports.push(var_decl.name.clone());
                }
                Statement::FuncDecl(func_decl) if func_decl.is_exported => {
                    exports.push(func_decl.name.clone());
                }
                Statement::ClassDecl(class_decl) if class_decl.is_exported => {
                    exports.push(class_decl.name.clone());
                }
                Statement::InterfaceDecl(interface_decl) if interface_decl.is_exported => {
                    exports.push(interface_decl.name.clone());
                }
                Statement::AnnotationDecl(annotation_decl) if annotation_decl.is_exported => {
                    exports.push(annotation_decl.name.clone());
                }
                _ => {}
            }
        }

        exports
    }

    /// Проверить, загружен ли модуль
    pub fn is_loaded(&self, module_path: &Path) -> bool {
        self.loaded_modules.contains_key(module_path)
    }

    /// Зарегистрировать экспортированные символы модуля в SymbolTable
    ///
    /// Проходит по всем statements и регистрирует экспортированные символы
    /// в указанной области видимости
    fn register_exports_in_symbol_table(
        ast: &Program,
        scope_id: ScopeId,
        symbol_table: &mut SymbolTable,
    ) -> Result<(), String> {
        for statement in &ast.statements {
            match statement {
                Statement::VarDecl(var_decl) if var_decl.is_exported => {
                    let symbol = Symbol::new(
                        var_decl.name.clone(),
                        SymbolKind::Variable,
                        true,
                        var_decl.span,
                    );
                    symbol_table
                        .define_in_scope(scope_id, symbol)
                        .map_err(|e| format!("Ошибка регистрации переменной '{}': {}", var_decl.name, e))?;
                }
                Statement::FuncDecl(func_decl) if func_decl.is_exported => {
                    let symbol = Symbol::new(
                        func_decl.name.clone(),
                        SymbolKind::Function,
                        true,
                        func_decl.span,
                    );
                    symbol_table
                        .define_in_scope(scope_id, symbol)
                        .map_err(|e| format!("Ошибка регистрации функции '{}': {}", func_decl.name, e))?;
                }
                Statement::ClassDecl(class_decl) if class_decl.is_exported => {
                    let symbol = Symbol::new(
                        class_decl.name.clone(),
                        SymbolKind::Class,
                        true,
                        class_decl.span,
                    );
                    symbol_table
                        .define_in_scope(scope_id, symbol)
                        .map_err(|e| format!("Ошибка регистрации класса '{}': {}", class_decl.name, e))?;
                }
                Statement::InterfaceDecl(interface_decl) if interface_decl.is_exported => {
                    let symbol = Symbol::new(
                        interface_decl.name.clone(),
                        SymbolKind::Interface,
                        true,
                        interface_decl.span,
                    );
                    symbol_table
                        .define_in_scope(scope_id, symbol)
                        .map_err(|e| format!("Ошибка регистрации интерфейса '{}': {}", interface_decl.name, e))?;
                }
                Statement::AnnotationDecl(annotation_decl) if annotation_decl.is_exported => {
                    let symbol = Symbol::new(
                        annotation_decl.name.clone(),
                        SymbolKind::Annotation,
                        true,
                        annotation_decl.span,
                    );
                    symbol_table
                        .define_in_scope(scope_id, symbol)
                        .map_err(|e| format!("Ошибка регистрации аннотации '{}': {}", annotation_decl.name, e))?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::env;

    #[test]
    fn test_extract_namespace_simple() {
        assert_eq!(
            ModuleResolver::extract_namespace("математика", None),
            "математика"
        );
    }

    #[test]
    fn test_extract_namespace_with_path() {
        assert_eq!(
            ModuleResolver::extract_namespace("путь/к/математика", None),
            "математика"
        );
    }

    #[test]
    fn test_extract_namespace_with_alias() {
        assert_eq!(
            ModuleResolver::extract_namespace("математика", Some("мат".to_string())),
            "мат"
        );
    }

    #[test]
    fn test_collect_exports() {
        use crate::ponos::ast::{VarDecl, FuncDecl, Statement};
        use crate::ponos::span::Span;

        let ast = Program {
            statements: vec![
                Statement::VarDecl(VarDecl {
                    name: "exported_var".to_string(),
                    type_annotation: None,
                    initializer: None,
                    is_exported: true,
                    span: Span::default(),
                }),
                Statement::VarDecl(VarDecl {
                    name: "private_var".to_string(),
                    type_annotation: None,
                    initializer: None,
                    is_exported: false,
                    span: Span::default(),
                }),
                Statement::FuncDecl(FuncDecl {
                    name: "exported_func".to_string(),
                    params: vec![],
                    body: vec![],
                    annotations: vec![],
                    is_exported: true,
                    span: Span::default(),
                }),
            ],
        };

        let exports = ModuleResolver::collect_exports(&ast);
        assert_eq!(exports.len(), 2);
        assert!(exports.contains(&"exported_var".to_string()));
        assert!(exports.contains(&"exported_func".to_string()));
        assert!(!exports.contains(&"private_var".to_string()));
    }

    #[test]
    fn test_module_resolver_creation() {
        let resolver = ModuleResolver::new();
        assert_eq!(resolver.loaded_modules.len(), 0);
    }

    #[test]
    fn test_load_simple_module() {
        // Создаем временный файл модуля
        let temp_dir = env::temp_dir().join("ponos_test_modules");
        fs::create_dir_all(&temp_dir).ok();

        let module_path = temp_dir.join("test_module.pns");
        fs::write(&module_path, "экспорт пер ПИ = 3.14;").unwrap();

        let mut resolver = ModuleResolver::new();
        let mut symbol_table = SymbolTable::new();
        let result = resolver.load_module(
            module_path.to_str().unwrap(),
            None,
            None,
            &mut symbol_table,
        );

        assert!(result.is_ok());
        let loaded = result.unwrap();
        assert_eq!(loaded.namespace, "test_module");
        assert_eq!(loaded.exports.len(), 1);
        assert_eq!(loaded.exports[0], "ПИ");

        // Проверяем, что символ зарегистрирован в SymbolTable
        let scope = symbol_table.get_scope(loaded.scope_id);
        let exports = scope.exported_symbols();
        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].name, "ПИ");

        // Очистка
        fs::remove_file(module_path).ok();
        fs::remove_dir(temp_dir).ok();
    }

    #[test]
    fn test_load_module_with_alias() {
        let temp_dir = env::temp_dir().join("ponos_test_alias");
        fs::create_dir_all(&temp_dir).ok();

        let module_path = temp_dir.join("математика.pns");
        fs::write(&module_path, "экспорт пер ПИ = 3.14;").unwrap();

        let mut resolver = ModuleResolver::new();
        let mut symbol_table = SymbolTable::new();
        let result = resolver.load_module(
            module_path.to_str().unwrap(),
            Some("мат".to_string()),
            None,
            &mut symbol_table,
        );

        assert!(result.is_ok());
        let loaded = result.unwrap();
        assert_eq!(loaded.namespace, "мат"); // Должен быть псевдоним

        // Очистка
        fs::remove_file(module_path).ok();
        fs::remove_dir(temp_dir).ok();
    }
}
