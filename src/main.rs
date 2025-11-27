use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result};
mod ponos;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 {
        _ = run_repl();
    } else if args.len() == 2 {
        run_file(args[1].clone());
    }
}

fn run_repl() -> Result<()> {
    let mut ponos = ponos::Ponos::new();

    let mut rl = DefaultEditor::new()?;

    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                ponos.run_source(line);
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}

fn run_file(file_name: String) {
    let file_content = fs::read_to_string(file_name).expect("Cannot open file");

    let mut ponos = ponos::Ponos::new();
    ponos.run_source(file_content);
}
