mod builtins;
use crate::builtins::change_directory;
use anyhow::Result;
use std::{
    collections::HashMap,
    env, fs,
    io::{BufRead, Write, stdout},
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
    str::FromStr,
};

#[derive(Debug, PartialEq)]
enum Builtin {
    CD,
    Exit,
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
            other => Err(RashError::InvalidVariant {
                input: other.to_string(),
            }),
        }
    }
}

fn main() -> std::process::ExitCode {
    // For now loading the PATH var once, before starting command capturing, because I need to think on what is an actual way to do this performantly. That is, the loading of new paths...
    let path_var = env::var("PATH").unwrap();
    let paths: Vec<PathBuf> = env::split_paths(&path_var).collect();
    let map = map_executables(paths).unwrap();

    display_prompt();
    loop {
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

            args.push(line);
            break;
        }

        let command_store: Vec<Vec<String>> = parse_to_command_store(&args.join(" "));

        for command in command_store {
            launch_command(&command, &map);
        }
        display_prompt(); // Then show prompt
    }
}

fn launch_command(argument_components: &[String], available_commands: &HashMap<String, PathBuf>) {
    let command = argument_components.first().unwrap();

    // Anything after the program command is assumed to be options
    let options = &argument_components[1..];

    // Try to parse the argument into a builtin. If the operation fails, we can assume that it's not a supported builtin, and can be tested agaisnt the PATH.
    match command.as_str().parse::<Builtin>() {
        Ok(command) => match command {
            Builtin::Exit => std::process::exit(0),
            Builtin::CD => change_directory::cd(&PathBuf::from(options[0].clone())),
        },
        Err(_) => {
            // Command seems to be external, try and find and execute it.
            if let Some(command) = available_commands.get(command) {
                run_command(command, options);
            } else {
                // If the process is not found..
                display_prompt(); // Then show prompt
            }
        }
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

pub fn parse_to_command_store(input: &str) -> Vec<Vec<String>> {
    input
        .split(';')
        .filter_map(|chunk| {
            let chunk = chunk.trim();
            if chunk.is_empty() {
                None
            } else {
                Some(chunk.split_whitespace().map(String::from).collect())
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_parse_command_store_single_command() {
        let input = "crackle";
        let expected = vec![vec!["crackle".to_string()]];
        assert_eq!(parse_to_command_store(input), expected);
    }

    #[test]
    fn test_parse_command_store_with_args() {
        let input = "echo why hello kitty cat!! hi hi!!";
        let expected = vec![vec![
            "echo".to_string(),
            "why".to_string(),
            "hello".to_string(),
            "kitty".to_string(),
            "cat!!".to_string(),
            "hi".to_string(),
            "hi!!".to_string(),
        ]];
        assert_eq!(parse_to_command_store(input), expected);
    }

    #[test]
    fn test_parse_command_store_multiple_commands() {
        let input = "ls -la;   pwd;echo bello recurse  ";
        let expected = vec![
            vec!["ls".to_string(), "-la".to_string()],
            vec!["pwd".to_string()],
            vec![
                "echo".to_string(),
                "bello".to_string(),
                "recurse".to_string(),
            ],
        ];
        assert_eq!(parse_to_command_store(input), expected);
    }

    #[test]
    fn test_parse_command_store_ignores_empty() {
        let input = ";  ;crackle  ; ; pop;";
        let expected = vec![vec!["crackle".to_string()], vec!["pop".to_string()]];
        assert_eq!(parse_to_command_store(input), expected);
    }

    #[test]
    fn test_builtin_from_str_valid() {
        assert_eq!(Builtin::from_str("cd").unwrap(), Builtin::CD);
        assert_eq!(Builtin::from_str("exit").unwrap(), Builtin::Exit);
    }

    #[test]
    fn test_builtin_from_str_invalid() {
        match Builtin::from_str("purrpurr") {
            Err(RashError::InvalidVariant { input }) => {
                assert_eq!(input, "purrpurr".to_string());
            }
            other => panic!("expected InvalidVariant, got {:?}", other),
        }
    }
}
