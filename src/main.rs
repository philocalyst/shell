mod builtins;
use crate::builtins::{change_directory, export};
use std::{
    collections::HashMap,
    env::{self, Args, args, current_dir},
    error::Error,
    fmt::Arguments,
    fs,
    io::{BufRead, IsTerminal, Write, stdout},
    os::unix::process::{self, CommandExt},
    path::{Path, PathBuf},
    str::FromStr,
};

#[derive(Debug, PartialEq)]
enum Builtin {
    CD,
    Exit,
    Export,
}

use std::io;

#[derive(Debug)]
pub enum RashError {
    /// Tried to parse an unknown variant
    InvalidVariant { input: String },
    /// Wrapper around any std::io::Error
    Io(io::Error),
}

impl From<io::Error> for RashError {
    fn from(err: io::Error) -> Self {
        RashError::Io(err)
    }
}

impl FromStr for Builtin {
    type Err = RashError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cd" => Ok(Builtin::CD),
            "exit" => Ok(Builtin::Exit),
            "export" => Ok(Builtin::Export),
            other => Err(RashError::InvalidVariant {
                input: other.to_string(),
            }),
        }
    }
}

use is_terminal;
fn main() -> std::process::ExitCode {
    // For now loading the PATH var once, before starting command capturing, because I need to think on what is an actual way to do this performantly. That is, the loading of new paths...
    let path_var = env::var("PATH").unwrap();
    let paths: Vec<PathBuf> = env::split_paths(&path_var).collect();
    let map = map_executables(paths).unwrap();

    if std::io::stdout().is_terminal() {
        display_prompt();
        'main: loop {
            let stdin = &io::stdin();

            // Get command line arguments
            let mut args = Vec::new();
            for line in stdin.lock().lines() {
                let line = line.unwrap();
                // If user enters a blank line, display a new prompt and reset args.
                if line.is_empty() {
                    display_prompt();
                    args.clear();
                    continue;
                }

                // Our continue conditions
                if line.contains('[') {
                    args.push(line);
                    continue;
                }

                args.push(line);
                break;
            }

            // Skip the program name

            let delimeter = "&&";
            let mut tokens: Vec<String> = Vec::new();
            for arg in args {
                for slice in arg.split(' ') {
                    tokens.push(slice.to_string());
                }
            }

            let command = tokens.first().unwrap();

            let options = &tokens[1..];

            match command.as_str() {
                // Exit is the leave keyword. Leave.
                "exit" => break 'main,
                "export" => export::export(&tokens),
                "cd" => change_directory::cd(&PathBuf::from(options[0].clone())),
                _ => (), // Ignore
            }

            // Command seems to be external, try and find it and execute it.
            if let Some(process) = map.get(command) {
                let code = run_command(&process, options);

                // Display error
                display_prompt(); // Then show prompt
            } else {
                // If the process is not found..
                println!("rash: Unknown command: {}", tokens[0]);
                display_prompt(); // Then show prompt
            }
        }

        return ExitCode::SUCCESS;
    } else {
        // No need to show prompt, not interactive.
        return ExitCode::SUCCESS;
    }
}

fn run_command(cmd: &PathBuf, args: &[String]) -> ExitCode {
    let status = Command::new(cmd)
        .args(args)
        // Effectively forking the process here, giving the child an inheritence of the terminals session.
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .expect("failed to spawn child");

    // Mapping status code to Exit Code
    match status.code() {
        Some(code) => ExitCode::from(code as u8),
        None => ExitCode::from(1),
    }
}

pub fn display_prompt() {
    // Display a prompt for the user :)
    print!("$ ");
    stdout().flush().unwrap();
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
