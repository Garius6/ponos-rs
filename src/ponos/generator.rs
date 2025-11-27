pub struct Generator {}

use crate::ponos::ast::UnaryOperator;

use super::ast::{AstNode, Expression, Statement};
use super::opcode::OpCode;
use super::value::Value;

pub struct GenContext {
    pub constants: Vec<Value>,
    pub opcodes: Vec<OpCode>,
}

impl Generator {
    pub fn new() -> Self {
        Generator {}
    }

    pub fn generate(&mut self, node: AstNode) -> GenContext {
        let mut context = GenContext {
            constants: Vec::new(),
            opcodes: Vec::new(),
        };
        match node {
            AstNode::Program(program) => {
                for stmt in program.statements {
                    self.emit_statement(stmt, &mut context);
                }
            }
            _ => panic!("Неверно сгенерировано ast"),
        }
        context
    }

    fn emit_statement(&mut self, stmt: Statement, ctx: &mut GenContext) {
        match stmt {
            Statement::Expression(e) => self.emit_expression(e, ctx),
            _ => unimplemented!(),
        }
    }

    fn emit_expression(&mut self, e: Expression, ctx: &mut GenContext) {
        match e {
            Expression::Number(n, _) => {
                ctx.constants.push(Value::Number(n));
                ctx.opcodes
                    .append(&mut vec![OpCode::Constant(ctx.constants.len() - 1)])
            }
            Expression::String(s, _) => {
                ctx.constants.push(Value::String(s));
                ctx.opcodes
                    .append(&mut vec![OpCode::Constant(ctx.constants.len() - 1)])
            }
            Expression::Boolean(b, _) => {
                ctx.constants.push(Value::Boolean(b));
                ctx.opcodes
                    .append(&mut vec![OpCode::Constant(ctx.constants.len() - 1)])
            }
            Expression::Identifier(_, _) => todo!(),
            Expression::Binary(binary_expr) => {
                self.emit_expression(binary_expr.left, ctx);
                self.emit_expression(binary_expr.right, ctx);
                let mut ops = match binary_expr.operator {
                    crate::ponos::ast::BinaryOperator::Add => vec![OpCode::Add],
                    crate::ponos::ast::BinaryOperator::Subtract => vec![OpCode::Sub],
                    crate::ponos::ast::BinaryOperator::Multiply => vec![OpCode::Mul],
                    crate::ponos::ast::BinaryOperator::Divide => vec![OpCode::Div],
                    crate::ponos::ast::BinaryOperator::Equal => vec![OpCode::Eql],
                    crate::ponos::ast::BinaryOperator::NotEqual => vec![OpCode::Eql, OpCode::Not],
                    crate::ponos::ast::BinaryOperator::Less => vec![OpCode::Less],
                    crate::ponos::ast::BinaryOperator::LessEqual => {
                        vec![OpCode::Greater, OpCode::Not]
                    }
                    crate::ponos::ast::BinaryOperator::Greater => vec![OpCode::Greater],
                    crate::ponos::ast::BinaryOperator::GreaterEqual => {
                        vec![OpCode::Less, OpCode::Not]
                    }
                    crate::ponos::ast::BinaryOperator::And => todo!(),
                    crate::ponos::ast::BinaryOperator::Or => todo!(),
                };
                ctx.opcodes.append(&mut ops);
            }
            Expression::Unary(unary_expr) => {
                self.emit_expression(unary_expr.operand, ctx);
                let op = match unary_expr.operator {
                    UnaryOperator::Negate => OpCode::Negate,
                    UnaryOperator::Not => OpCode::Not,
                };
                ctx.opcodes.push(op);
            }
            Expression::Call(call_expr) => todo!(),
            Expression::FieldAccess(field_access_expr) => todo!(),
            Expression::Lambda(lambda_expr) => todo!(),
            Expression::This(span) => todo!(),
            Expression::Super(_, span) => todo!(),
        }
    }
}
