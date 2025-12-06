use crate::ponos::span::Span;

#[derive(Debug, Clone)]
pub enum AstNode {
    Program(Program),
    Statement(Statement),
    Expression(Expression),
}

#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Statement {
    VarDecl(VarDecl),
    FuncDecl(FuncDecl),
    ClassDecl(ClassDecl),
    InterfaceDecl(InterfaceDecl),
    AnnotationDecl(AnnotationDecl),
    Import(ImportStatement),
    ModuleBlock(ModuleBlock), // Блок кода модуля с пространством имен
    If(IfStatement),
    While(WhileStatement),
    Return(ReturnStatement),
    Assignment(AssignmentStatement),
    Expression(Expression),
}

#[derive(Debug, Clone)]
pub struct VarDecl {
    pub name: String,
    pub type_annotation: Option<String>,
    pub initializer: Option<Expression>,
    pub is_exported: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FuncDecl {
    pub name: String,
    pub params: Vec<Parameter>,
    pub body: Vec<Statement>,
    pub annotations: Vec<Annotation>,
    pub is_exported: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub type_annotation: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Annotation {
    pub name: String,
    pub args: Vec<AnnotationArgument>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum AnnotationArgument {
    Positional(Expression),
    Named { name: String, value: Expression },
}

#[derive(Debug, Clone)]
pub struct IfStatement {
    pub condition: Expression,
    pub then_branch: Vec<Statement>,
    pub else_branch: Option<Vec<Statement>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct WhileStatement {
    pub condition: Expression,
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ReturnStatement {
    pub value: Option<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum AssignmentTarget {
    Identifier(String),
    FieldAccess(Box<Expression>, String), // объект.поле
}

#[derive(Debug, Clone)]
pub struct AssignmentStatement {
    pub target: AssignmentTarget,
    pub value: Expression,
    pub span: Span,
}

// Классы и интерфейсы

#[derive(Debug, Clone)]
pub struct ClassDecl {
    pub name: String,
    pub extends: Option<String>, // Родительский класс
    pub implements: Vec<String>, // Реализуемые интерфейсы
    pub members: Vec<ClassMember>,
    pub annotations: Vec<Annotation>,
    pub is_exported: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ClassMember {
    Field {
        name: String,
        type_annotation: Option<String>,
    },
    Method(FuncDecl),
    Constructor(ConstructorDecl),
}

#[derive(Debug, Clone)]
pub struct ConstructorDecl {
    pub params: Vec<Parameter>,
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct InterfaceDecl {
    pub name: String,
    pub methods: Vec<MethodSignature>,
    pub is_exported: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct MethodSignature {
    pub name: String,
    pub params: Vec<Parameter>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AnnotationDecl {
    pub name: String,
    pub body: Vec<Statement>,
    pub is_exported: bool,
    pub span: Span,
}

// Модули и импорты

#[derive(Debug, Clone)]
pub struct ImportStatement {
    pub path: String,
    pub alias: Option<String>, // Переименование: использовать "модуль" как псевдоним
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ModuleBlock {
    pub namespace: String,          // Имя пространства имен
    pub statements: Vec<Statement>, // Statements модуля
    pub span: Span,
}

// Выражения

#[derive(Debug, Clone)]
pub enum Expression {
    // Литералы
    Number(f64, Span),
    String(String, Span),
    Boolean(bool, Span),

    // Идентификаторы
    Identifier(String, Span),

    // Бинарные операции
    Binary(Box<BinaryExpr>),

    // Унарные операции
    Unary(Box<UnaryExpr>),

    // Вызов функции
    Call(Box<CallExpr>),

    // Доступ к полю объекта
    FieldAccess(Box<FieldAccessExpr>),

    // Доступ к символу модуля (модуль.символ)
    ModuleAccess(Box<ModuleAccessExpr>),

    // Замыкание/лямбда
    Lambda(Box<LambdaExpr>),

    // Специальные
    This(Span),
    Super(String, Span), // Super(method_name, span)
}

impl Expression {
    pub fn span(&self) -> Span {
        match self {
            Expression::Number(_, s) => *s,
            Expression::String(_, s) => *s,
            Expression::Boolean(_, s) => *s,
            Expression::Identifier(_, s) => *s,
            Expression::Binary(e) => e.span,
            Expression::Unary(e) => e.span,
            Expression::Call(e) => e.span,
            Expression::FieldAccess(e) => e.span,
            Expression::ModuleAccess(e) => e.span,
            Expression::Lambda(e) => e.span,
            Expression::This(s) => *s,
            Expression::Super(_, s) => *s,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BinaryExpr {
    pub left: Expression,
    pub operator: BinaryOperator,
    pub right: Expression,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    // Арифметические
    Add,      // +
    Subtract, // -
    Multiply, // *
    Divide,   // /

    // Сравнения
    Equal,        // ==
    NotEqual,     // !=
    Less,         // <
    LessEqual,    // <=
    Greater,      // >
    GreaterEqual, // >=

    // Логические
    And, // и
    Or,  // или
}

impl BinaryOperator {
    /// Возвращает приоритет оператора (больше = выше приоритет)
    pub fn precedence(&self) -> u8 {
        match self {
            BinaryOperator::Or => 1,
            BinaryOperator::And => 2,
            BinaryOperator::Equal | BinaryOperator::NotEqual => 3,
            BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual => 4,
            BinaryOperator::Add | BinaryOperator::Subtract => 5,
            BinaryOperator::Multiply | BinaryOperator::Divide => 6,
        }
    }

    /// Все операторы левоассоциативны
    pub fn is_left_associative(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone)]
pub struct UnaryExpr {
    pub operator: UnaryOperator,
    pub operand: Expression,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Negate, // -
    Not,    // !
}

#[derive(Debug, Clone)]
pub struct CallExpr {
    pub callee: Expression,
    pub arguments: Vec<Expression>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FieldAccessExpr {
    pub object: Expression,
    pub field: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ModuleAccessExpr {
    pub namespace: String, // Имя пространства имен (математика, мат)
    pub symbol: String,    // Имя символа (корень, ПИ)
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct LambdaExpr {
    pub params: Vec<Parameter>,
    pub body: Vec<Statement>,
    pub span: Span,
}
