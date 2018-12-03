//! Command line prompt for the master mode
//!

use master::cmd::Cmd;
use rustyline::Editor;
use rustyline::error::ReadlineError;

pub struct Prompt {
    editor: Editor<()>,
}

static HISTORY_FILE: &str = ".master.history.txt";


pub fn prompt() -> Prompt {
    let mut editor = Editor::<()>::new();
    if let Err(_) = editor.load_history(HISTORY_FILE) {
        trace!("No previous history for master prompt");
    }
    Prompt { editor }
}


impl Iterator for Prompt {
    type Item = Cmd;

    fn next(&mut self) -> Option<Cmd> {
        loop {
            match self.editor.readline("> ") {
                Ok(line) => {
                    match Cmd::parse(line) {
                        Ok(cmd) => {
                            self.editor.save_history(HISTORY_FILE);
                            return Some(cmd);
                        }
                        Err(e) => eprintln!("Cannot parse the input: {}", e),
                    }
                }
                Err(err) => {
                    println!("Master: {}", err);
                    return None;
                }
            }
        }
    }
}
