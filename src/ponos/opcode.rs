#[derive(Debug, PartialEq, Clone, Copy)]
pub enum OpCode {
    Constant(usize),
    Pop, // Удалить значение с вершины стека
    Dup, // Дублировать значение на вершине стека

    // Математика
    Negate,
    Add,
    Sub,
    Mul,
    Div,
    Mod,

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

    // ООП
    Class,
    Inherit,             // Установить родительский класс (pop superclass, pop subclass, push subclass)
    DefineMethod(usize), // Добавить метод в класс (имя в константах)
    GetProperty, // Получить свойство экземпляра
    SetProperty, // Установить свойство экземпляра
    GetSuper,    // Получить метод родительского класса

    // Индексирование и коллекции
    GetIndex, // Получить элемент по индексу (2 значения на стеке: объект, индекс)

    // переменные
    DefineGlobal(usize),
    SetGlobal(usize),
    GetGlobal(usize),

    Halt, // Данный опкод не никак не обрабатывается и нужен только чтобы jump'у в конце выражения
          // было куда переходить
}
