mod ast;
mod generator;
mod module;
mod opcode;
mod parser;
mod span;
mod symbol_table;
mod value;
mod vm;

pub struct Ponos {
    parser: parser::PonosParser,
    vm: vm::VM,
    generator: generator::Generator,
}

impl Ponos {
    pub fn new() -> Self {
        let parser = parser::PonosParser::new();

        return Ponos {
            parser: parser,
            vm: vm::VM::new(),
            generator: generator::Generator::new(),
        };
    }

    pub fn run_source(&mut self, source: String) {
        println!("source:\n{}", source);

        let ast = self.parser.parse(source.clone()).unwrap();
        println!("ast:\n{:#?}", ast);

        let ctx = self.generator.generate(ast::AstNode::Program(ast));
        println!("opcodes:\n{:#?}", ctx.opcodes);

        self.vm.execute(ctx.opcodes);
    }
}
