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
    Closure(usize, usize),
    GetUpvalue(usize),
    SetUpvalue(usize),
    CloseUpvalues(usize),

    // поток выполнения
    Jump(usize),        // Безусловный переход на абсолютный адрес
    JumpIfTrue(usize),  // Переход если вершина стека true
    JumpIfFalse(usize), // Переход если вершина стека false
    Call(usize),
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

    Halt, // Данный опкод не никак не обрабатывается и нужен только чтобы jump'у в конце выражения
          // было куда переходить
}
