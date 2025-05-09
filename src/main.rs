use std::{
    env::{Args, args, current_dir},
    fmt::Arguments,
    io::IsTerminal,
};

use std::io;

use is_terminal;
fn main() {
    if std::io::stdout().is_terminal() {
        // Display a prompt for the user.
        print!("{:?}", current_dir());

        // Get command line arguments
        let mut args: Args = std::env::args();

        // Skip the program name
        args.next();

        let delimeter = ',';
        let mut tokens: Vec<String> = Vec::new();
        while let Some(arg) = args.next() {
            for piece in arg.split(delimeter) {
                tokens.push(piece.to_string());
            }
        }

        print!("{:?}", tokens);
    } else {
        // No need to show prompt, not interactive.
    }
}
