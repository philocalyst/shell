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
    } else {
        // No need to show prompt, not interactive.
    }
}
