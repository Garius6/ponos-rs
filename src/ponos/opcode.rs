#[derive(Debug, PartialEq, Clone, Copy)]
pub enum OpCode {
    Constant(usize),

    // Математика
    Negate,
    Add,
    Sub,
    Mul,
    Div,

    // логические операторы
    True_,
    False_,

    Eql,
    Not,

    Greater,
    Less,

    // локальные переменные
    DefineLocal(usize),
    GetLocal(usize),
    SetLocal(usize),

    // замыкания
    Closure,
    GetUpvalue,
    SetUpvalue,
    CloseUpvalues,

    // поток выполнения
    Jump,
    JumpIfTrue,
    JumpIfFalse,
    Call,
    Return_,

    // работа со стеком
    Pop,
    Push,

    // ООП
    Class,
    Instance,    // Создать экземпляр класса
    GetProperty, // Получить свойство экземпляра
    SetProperty, // Установить свойство экземпляра
    Invoke,      // Вызвать метод на экземпляре
    GetSuper,    // Получить метод родительского класса

    // переменные
    DefineGlobal(usize),
    SetGlobal(usize),
    GetGlobal(usize),
}
