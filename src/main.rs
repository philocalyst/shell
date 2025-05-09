use std::{
    collections::HashMap,
    env::{self, Args, args, current_dir},
    error::Error,
    fmt::{Arguments, Result},
    fs,
    io::{BufRead, IsTerminal},
    os::unix::process,
    path::{Path, PathBuf},
    process::{Command, ExitCode, ExitStatus},
};

use std::io;

use is_terminal;
fn main() -> std::process::ExitCode {
    // For now loading the PATH var once, before starting command capturing, because I need to think on what is an actual way to do this performantly. That is, the loading of new paths...
    let path_var = env::var("PATH").unwrap();
    let paths: Vec<PathBuf> = env::split_paths(&path_var).collect();
    let map = map_executables(paths).unwrap();

    if std::io::stdout().is_terminal() {
        // Display a prompt for the user :)
        print!("{:?}", current_dir());

        loop {
            let stdin = &io::stdin();

            // Get command line arguments
            let mut args = Vec::new();
            for line in stdin.lock().lines() {
                println!("{:?}", &line);
                args.push(line.unwrap());
            }

            // Skip the program name

            let delimeter = "&&";
            let mut tokens: Vec<String> = Vec::new();
            while let Some(arg) = args.clone().into_iter().next() {
                for piece in arg.split(delimeter) {
                    tokens.push(piece.to_string());
                }
            }
            println!("{:?}", args);

            if let Some(process) = map.get(&tokens[0]) {
                let output = Command::new(process).output().unwrap();
                println!("{:?}", output);
            } else {
                // If the process is not found..
                println!("rash: Unknown command: {}", tokens[0])
            }
            break;
        }

        return ExitCode::SUCCESS;
    } else {
        // No need to show prompt, not interactive.
        return ExitCode::SUCCESS;
    }
}

pub fn map_executables<I, P>(dirs: I) -> io::Result<HashMap<String, PathBuf>>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut map = HashMap::new();

    for dir in dirs {
        let dir = dir.as_ref();
        if !dir.is_dir() {
            continue;
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?; // Pass up any entry reading errors
            let path = entry.path();
            if !path.is_file() {
                continue; // We're ignoring any subdirs or symlinks.
            }
            if let Some(name_os) = path.file_name() {
                let name = name_os.to_string_lossy().into_owned();
                // Don't insert if an entry is already present at key
                map.entry(name).or_insert(path);
            }
        }
    }

    Ok(map)
}
