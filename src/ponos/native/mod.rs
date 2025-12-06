pub mod fs;
pub mod io;
pub mod system;

use crate::ponos::vm::VM;
use std::collections::HashMap;

/// Реестр нативных модулей
pub struct NativeModuleRegistry {
    modules: HashMap<String, NativeModule>,
}

/// Нативный модуль с экспортированными функциями
pub struct NativeModule {
    pub name: String,
    pub exports: Vec<String>,
}

impl NativeModuleRegistry {
    /// Создать новый реестр с зарегистрированными нативными модулями
    pub fn new() -> Self {
        let mut registry = NativeModuleRegistry {
            modules: HashMap::new(),
        };

        // Регистрируем встроенные модули
        registry.register_module(NativeModule {
            name: "стд/ввод_вывод".to_string(),
            exports: vec!["вывести".to_string(), "ввести".to_string()],
        });

        registry.register_module(NativeModule {
            name: "стд/фс".to_string(),
            exports: vec![
                "читать".to_string(),
                "писать".to_string(),
                "существует".to_string(),
                "удалить".to_string(),
            ],
        });

        registry.register_module(NativeModule {
            name: "стд/система".to_string(),
            exports: vec!["выполнить".to_string()],
        });

        registry
    }

    /// Зарегистрировать нативный модуль
    fn register_module(&mut self, module: NativeModule) {
        self.modules.insert(module.name.clone(), module);
    }

    /// Проверить, является ли путь нативным модулем
    pub fn is_native_module(&self, path: &str) -> bool {
        self.modules.contains_key(path)
    }

    /// Получить информацию о нативном модуле
    pub fn get_module(&self, path: &str) -> Option<&NativeModule> {
        self.modules.get(path)
    }

    /// Зарегистрировать функции нативного модуля в VM
    pub fn register_module_in_vm(&self, module_path: &str, namespace: &str, vm: &mut VM) -> Result<(), String> {
        let module = self.get_module(module_path)
            .ok_or_else(|| format!("Нативный модуль '{}' не найден", module_path))?;

        // Регистрируем функции с манглированными именами
        match module_path {
            "стд/ввод_вывод" => {
                for export in &module.exports {
                    let mangled_name = format!("{}::{}", namespace, export);
                    match export.as_str() {
                        "вывести" => {
                            vm.register_and_define(&mangled_name, io::io_print);
                        }
                        "ввести" => {
                            vm.register_and_define(&mangled_name, io::io_input);
                        }
                        _ => {}
                    }
                }
            }
            "стд/фс" => {
                for export in &module.exports {
                    let mangled_name = format!("{}::{}", namespace, export);
                    match export.as_str() {
                        "читать" => {
                            vm.register_and_define(&mangled_name, fs::fs_read);
                        }
                        "писать" => {
                            vm.register_and_define(&mangled_name, fs::fs_write);
                        }
                        "существует" => {
                            vm.register_and_define(&mangled_name, fs::fs_exists);
                        }
                        "удалить" => {
                            vm.register_and_define(&mangled_name, fs::fs_delete);
                        }
                        _ => {}
                    }
                }
            }
            "стд/система" => {
                for export in &module.exports {
                    let mangled_name = format!("{}::{}", namespace, export);
                    match export.as_str() {
                        "выполнить" => {
                            vm.register_and_define(&mangled_name, system::sys_execute);
                        }
                        _ => {}
                    }
                }
            }
            _ => {
                return Err(format!("Неизвестный нативный модуль '{}'", module_path));
            }
        }

        Ok(())
    }
}
