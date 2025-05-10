mod builtins;
use crate::builtins::{change_directory, export};
use anyhow::Result;
use std::{
    collections::HashMap,
    env::{self, Args, args, current_dir},
    error::Error,
    fmt::Arguments,
    fs,
    io::{BufRead, IsTerminal, Read, Write, stdout},
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

use std::process::{Command, ExitCode, Stdio};

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

            // Try to parse the argument into a builtin. If the operation fails, we can assume that it's not a supported builtin, and can be tested agaisnt the PATH.
            match command.as_str().parse::<Builtin>() {
                Ok(command) => {
                    match command {
                        Builtin::Exit => break 'main,
                        Builtin::Export => export::export(&tokens),
                        Builtin::CD => change_directory::cd(&PathBuf::from(options[0].clone())),
                    }
                    display_prompt(); // Then show prompt
                }
                Err(_) => {
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
            }
        }

        return ExitCode::SUCCESS;
    } else {
        // No need to show prompt, not interactive.
        return ExitCode::SUCCESS;
    }
}

fn run_command(cmd: &PathBuf, args: &[String]) -> Result<(ExitCode, String)> {
    // Create a child process with the command and args
    let mut baby = Command::new(cmd)
        .args(args)
        // Effectively forking the process here, giving the child an inheritence of the terminals session.
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    // Take ownership of the output as a pipe
    let mut stdout_pipe = baby.stdout.take().expect("Stdout was piped but was empty");

    // Read the output into a vector buffer of u8
    let mut output_buffer: Vec<u8> = Vec::new();
    stdout_pipe.read_to_end(&mut output_buffer);

    // Get the result status to pass upwards
    let resulting_status = baby.wait()?;
    let exit_code = resulting_status.code().unwrap_or(1) as u8;

    // Read the vector buffer to an output string
    let output_string = String::from_utf8(output_buffer)?;

    Ok((ExitCode::from(exit_code), output_string))
}

pub fn display_prompt() {
    // Display a prompt for the user :)
    print!("$ ");
    stdout().flush().unwrap();
}

pub fn map_executables<I, P>(dirs: I) -> Result<HashMap<String, PathBuf>>
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
