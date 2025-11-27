use crate::ponos::{opcode, value};

pub struct VM {
    stack: Vec<value::Value>,
}

impl<'a> VM {
    pub fn new() -> Self {
        VM { stack: Vec::new() }
    }

    pub fn execute(&self, opcodes: Vec<opcode::OpCode>) {
        ()
    }
}
