use std::{
    collections::HashMap,
    env::{self, Args, args, current_dir},
    error::Error,
    fmt::{Arguments, Result},
    fs,
    io::IsTerminal,
    path::PathBuf,
    process::{self, ExitCode, ExitStatus},
};

use std::io;

use is_terminal;
fn main() -> std::process::ExitCode {
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

        let path_var = env::var("PATH").unwrap();
        let paths: Vec<PathBuf> = env::split_paths(&path_var).collect();
        create_path_map(paths);

        print!("{:?}", tokens);
        return ExitCode::SUCCESS;
    } else {
        // No need to show prompt, not interactive.
        return ExitCode::SUCCESS;
    }
}

fn create_path_map(path_components: Vec<PathBuf>) -> HashMap<String, PathBuf> {
    // Map the name of an executable to its source path
    let mut executable_map: HashMap<String, PathBuf> = HashMap::new();

    for path in path_components {
        if path.is_dir() {
            let processes = fs::read_dir(path).unwrap();
            processes.map(|entry| {
                executable_map.insert(
                    entry
                        .unwrap()
                        .path()
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                    entry.unwrap().path(),
                )
            });
        }
    }
    executable_map
}
