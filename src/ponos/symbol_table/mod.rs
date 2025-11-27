use crate::ponos::span::Span;
use std::collections::HashMap;

/// Тип символа
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Variable,
    Function,
    Class,
    Interface,
    Annotation,
}

/// Идентификатор области видимости
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScopeId(pub usize);

/// Символ в таблице символов
/// Единое представление для переменных, функций, классов и т.д.
#[derive(Debug, Clone)]
pub struct Symbol {
    /// Имя символа
    pub name: String,
    /// Тип символа
    pub kind: SymbolKind,
    /// Экспортируется ли символ
    pub is_exported: bool,
    /// Позиция в исходном коде
    pub span: Span,
    // TODO: В будущем добавить type_info для типчекера
}

impl Symbol {
    /// Создать новый символ
    pub fn new(name: String, kind: SymbolKind, is_exported: bool, span: Span) -> Self {
        Symbol {
            name,
            kind,
            is_exported,
            span,
        }
    }
}

/// Область видимости
#[derive(Debug, Clone)]
pub struct Scope {
    /// Идентификатор этой области
    pub id: ScopeId,
    /// Родительская область видимости
    pub parent: Option<ScopeId>,
    /// Символы в этой области
    symbols: HashMap<String, Symbol>,
}

impl Scope {
    /// Создать новую область видимости
    pub fn new(id: ScopeId, parent: Option<ScopeId>) -> Self {
        Scope {
            id,
            parent,
            symbols: HashMap::new(),
        }
    }

    /// Добавить символ в область видимости
    pub fn define(&mut self, symbol: Symbol) -> Result<(), String> {
        if self.symbols.contains_key(&symbol.name) {
            return Err(format!(
                "Символ '{}' уже определён в этой области",
                symbol.name
            ));
        }
        self.symbols.insert(symbol.name.clone(), symbol);
        Ok(())
    }

    /// Найти символ в этой области (без поиска в родителях)
    pub fn lookup_local(&self, name: &str) -> Option<&Symbol> {
        self.symbols.get(name)
    }

    /// Получить все символы
    pub fn symbols(&self) -> &HashMap<String, Symbol> {
        &self.symbols
    }

    /// Получить экспортированные символы
    pub fn exported_symbols(&self) -> Vec<&Symbol> {
        self.symbols.values().filter(|s| s.is_exported).collect()
    }
}

/// Единая таблица символов для всей программы
/// Управляет областями видимости и символами
#[derive(Debug)]
pub struct SymbolTable {
    /// Все области видимости
    scopes: Vec<Scope>,
    /// Счётчик для генерации ScopeId
    next_scope_id: usize,
    /// Текущая активная область видимости
    current_scope: ScopeId,
}

impl SymbolTable {
    /// Создать новую таблицу символов с глобальной областью видимости
    pub fn new() -> Self {
        let global_scope_id = ScopeId(0);
        let global_scope = Scope::new(global_scope_id, None);

        SymbolTable {
            scopes: vec![global_scope],
            next_scope_id: 1,
            current_scope: global_scope_id,
        }
    }

    /// Создать новую область видимости
    pub fn push_scope(&mut self) -> ScopeId {
        let scope_id = ScopeId(self.next_scope_id);
        self.next_scope_id += 1;

        let scope = Scope::new(scope_id, Some(self.current_scope));
        self.scopes.push(scope);
        self.current_scope = scope_id;

        scope_id
    }

    /// Закрыть текущую область видимости
    pub fn pop_scope(&mut self) {
        let current = self.get_scope(self.current_scope);
        if let Some(parent_id) = current.parent {
            self.current_scope = parent_id;
        }
    }

    /// Получить область видимости по ID
    pub fn get_scope(&self, id: ScopeId) -> &Scope {
        &self.scopes[id.0]
    }

    /// Получить мутабельную область видимости по ID
    pub fn get_scope_mut(&mut self, id: ScopeId) -> &mut Scope {
        &mut self.scopes[id.0]
    }

    /// Определить символ в текущей области видимости
    pub fn define(&mut self, symbol: Symbol) -> Result<(), String> {
        self.get_scope_mut(self.current_scope).define(symbol)
    }

    /// Определить символ в конкретной области видимости
    pub fn define_in_scope(&mut self, scope_id: ScopeId, symbol: Symbol) -> Result<(), String> {
        self.get_scope_mut(scope_id).define(symbol)
    }

    /// Найти символ (с поиском в родительских областях)
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        let mut current_id = self.current_scope;

        loop {
            let scope = self.get_scope(current_id);

            if let Some(symbol) = scope.lookup_local(name) {
                return Some(symbol);
            }

            // Поднимаемся в родительскую область
            match scope.parent {
                Some(parent_id) => current_id = parent_id,
                None => return None,
            }
        }
    }

    /// Найти символ в конкретной области (без поиска в родителях)
    pub fn lookup_in_scope(&self, scope_id: ScopeId, name: &str) -> Option<&Symbol> {
        self.get_scope(scope_id).lookup_local(name)
    }

    /// Получить текущий scope ID
    pub fn current_scope(&self) -> ScopeId {
        self.current_scope
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_kind_equality() {
        assert_eq!(SymbolKind::Variable, SymbolKind::Variable);
        assert_ne!(SymbolKind::Variable, SymbolKind::Function);
    }

    #[test]
    fn test_symbol_creation() {
        let symbol = Symbol::new(
            "test".to_string(),
            SymbolKind::Function,
            true,
            Span::new(0, 10),
        );
        assert_eq!(symbol.name, "test");
        assert_eq!(symbol.kind, SymbolKind::Function);
        assert_eq!(symbol.is_exported, true);
    }

    #[test]
    fn test_scope_define_and_lookup() {
        let mut scope = Scope::new(ScopeId(0), None);

        let symbol = Symbol::new(
            "foo".to_string(),
            SymbolKind::Variable,
            false,
            Span::new(0, 10),
        );

        assert!(scope.define(symbol).is_ok());
        assert!(scope.lookup_local("foo").is_some());
        assert!(scope.lookup_local("bar").is_none());
    }

    #[test]
    fn test_scope_duplicate_symbol() {
        let mut scope = Scope::new(ScopeId(0), None);

        let symbol1 = Symbol::new(
            "foo".to_string(),
            SymbolKind::Variable,
            false,
            Span::new(0, 10),
        );
        let symbol2 = Symbol::new(
            "foo".to_string(),
            SymbolKind::Function,
            false,
            Span::new(10, 20),
        );

        assert!(scope.define(symbol1).is_ok());
        assert!(scope.define(symbol2).is_err());
    }

    #[test]
    fn test_scope_exported_symbols() {
        let mut scope = Scope::new(ScopeId(0), None);

        scope
            .define(Symbol::new(
                "foo".to_string(),
                SymbolKind::Variable,
                false,
                Span::new(0, 10),
            ))
            .unwrap();
        scope
            .define(Symbol::new(
                "bar".to_string(),
                SymbolKind::Function,
                true,
                Span::new(10, 20),
            ))
            .unwrap();
        scope
            .define(Symbol::new(
                "baz".to_string(),
                SymbolKind::Class,
                true,
                Span::new(20, 30),
            ))
            .unwrap();

        let exported = scope.exported_symbols();
        assert_eq!(exported.len(), 2);
        assert!(exported.iter().any(|s| s.name == "bar"));
        assert!(exported.iter().any(|s| s.name == "baz"));
        assert!(!exported.iter().any(|s| s.name == "foo"));
    }

    #[test]
    fn test_symbol_table_creation() {
        let table = SymbolTable::new();
        assert_eq!(table.scopes.len(), 1); // global scope
    }

    #[test]
    fn test_symbol_table_define_and_lookup() {
        let mut table = SymbolTable::new();

        let symbol = Symbol::new(
            "test".to_string(),
            SymbolKind::Variable,
            false,
            Span::new(0, 10),
        );

        assert!(table.define(symbol).is_ok());
        assert!(table.lookup("test").is_some());
        assert!(table.lookup("nonexistent").is_none());
    }

    #[test]
    fn test_symbol_table_scopes() {
        let mut table = SymbolTable::new();

        // Определяем в глобальной области
        table
            .define(Symbol::new(
                "global".to_string(),
                SymbolKind::Variable,
                false,
                Span::new(0, 10),
            ))
            .unwrap();

        // Создаём вложенную область
        table.push_scope();
        table
            .define(Symbol::new(
                "local".to_string(),
                SymbolKind::Variable,
                false,
                Span::new(10, 20),
            ))
            .unwrap();

        // Должны видеть оба символа
        assert!(table.lookup("global").is_some());
        assert!(table.lookup("local").is_some());

        // Закрываем область
        table.pop_scope();

        // Теперь видим только глобальный
        assert!(table.lookup("global").is_some());
        assert!(table.lookup("local").is_none());
    }

    #[test]
    fn test_symbol_table_shadowing() {
        let mut table = SymbolTable::new();

        table
            .define(Symbol::new(
                "x".to_string(),
                SymbolKind::Variable,
                false,
                Span::new(0, 10),
            ))
            .unwrap();

        table.push_scope();
        table
            .define(Symbol::new(
                "x".to_string(),
                SymbolKind::Variable,
                false,
                Span::new(10, 20),
            ))
            .unwrap();

        let found = table.lookup("x");
        assert!(found.is_some());
        assert_eq!(found.unwrap().span.start, 10); // Должен найти локальный
    }
}
