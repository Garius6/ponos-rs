mod ast;
mod generator;
mod module;
mod name_resolver;
pub mod native;
mod opcode;
mod parser;
mod span;
pub mod stdlib;
mod symbol_table;
mod value;
mod vm;

use module::{ModuleResolver, merge_module_ast};
use name_resolver::NameResolver;
use std::path::PathBuf;
use symbol_table::SymbolTable;

pub struct Ponos {
    parser: parser::PonosParser,
    vm: vm::VM,
    generator: generator::Generator,
    module_resolver: ModuleResolver,
    name_resolver: NameResolver,
    symbol_table: SymbolTable,
}

impl Ponos {
    pub fn new() -> Self {
        let parser = parser::PonosParser::new();
        let vm = vm::VM::new();

        return Ponos {
            parser: parser,
            vm: vm,
            generator: generator::Generator::new(),
            module_resolver: ModuleResolver::new(),
            name_resolver: NameResolver::new(),
            symbol_table: SymbolTable::new(),
        };
    }

    pub fn run_source(&mut self, source: String) {
        self.run_source_with_file(source, None);
    }

    /// Запустить исходный код с указанием файла (для импортов)
    pub fn run_source_with_file(&mut self, source: String, file_path: Option<PathBuf>) {
        println!("source:\n{}", source);

        // 1. Парсинг
        let mut ast = self.parser.parse(source.clone()).unwrap();
        println!("ast:\n{:#?}", ast);

        // 2. Обработка импортов и загрузка модулей
        self.process_imports(&mut ast, file_path.as_deref());

        // 3. Разрешение имён (преобразование FieldAccess в ModuleAccess)
        self.name_resolver
            .resolve(&mut ast, &self.symbol_table)
            .expect("Ошибка разрешения имён");
        println!("ast после разрешения имён:\n{:#?}", ast);

        // 4. Генерация байткода
        let mut ctx = self.generator.generate(ast::AstNode::Program(ast));
        println!("opcodes:\n{:#?}", ctx.opcodes);
        println!("constants:\n{:#?}", ctx.constants);

        // 5. Выполнение
        self.vm.execute(ctx.opcodes, &mut ctx.constants);
        println!("vm stack:\n{:#?}", self.vm.stack);
    }

    /// Обработать импорты в AST: загрузить модули и зарегистрировать их
    fn process_imports(&mut self, ast: &mut ast::Program, from_file: Option<&std::path::Path>) {
        use ast::Statement;
        use span::Span;
        use symbol_table::Symbol;

        // Собираем все импорты из AST
        let mut imports = Vec::new();
        for stmt in &ast.statements {
            if let Statement::Import(import) = stmt {
                imports.push((import.path.clone(), import.alias.clone()));
            }
        }

        // Загружаем каждый модуль
        for (path, alias) in imports {
            match self.module_resolver.load_module(
                &path,
                alias.clone(),
                from_file,
                &mut self.symbol_table,
            ) {
                Ok(loaded_module) => {
                    println!(
                        "Загружен модуль: {} (пространство имён: {})",
                        path, loaded_module.namespace
                    );

                    // Если это нативный модуль, регистрируем его функции в VM
                    if loaded_module.file_path.to_str().unwrap_or("").starts_with("<native:") {
                        let native_registry = self.module_resolver.native_registry();
                        if let Err(e) = native_registry.register_module_in_vm(
                            &path,
                            &loaded_module.namespace,
                            &mut self.vm,
                        ) {
                            eprintln!(
                                "Предупреждение: не удалось зарегистрировать нативные функции для '{}': {}",
                                path, e
                            );
                        }
                    }

                    // Регистрируем пространство имён как Symbol::Module в текущей области
                    let module_symbol = Symbol::new_module(
                        loaded_module.namespace.clone(),
                        loaded_module.scope_id,
                        Span::default(),
                    );

                    if let Err(e) = self.symbol_table.define(module_symbol) {
                        eprintln!(
                            "Предупреждение: не удалось зарегистрировать модуль '{}': {}",
                            loaded_module.namespace, e
                        );
                    }

                    // Добавляем AST модуля в основной AST
                    merge_module_ast(ast, loaded_module);
                }
                Err(e) => {
                    eprintln!("Ошибка загрузки модуля '{}': {}", path, e);
                }
            }
        }

        // КРИТИЧНО: Переупорядочиваем statements чтобы ModuleBlocks были первыми
        // Это нужно чтобы глобальные переменные модулей определились до их использования
        let mut module_blocks = Vec::new();
        let mut other_statements = Vec::new();

        for stmt in ast.statements.drain(..) {
            match stmt {
                Statement::ModuleBlock(_) => module_blocks.push(stmt),
                _ => other_statements.push(stmt),
            }
        }

        // Сначала ModuleBlocks, потом остальное
        ast.statements.extend(module_blocks);
        ast.statements.extend(other_statements);
    }
}
