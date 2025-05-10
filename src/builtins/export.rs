use clap::builder::ValueParser;
use clap::{Arg, Command};
use std::collections::HashMap;

fn parse_kv(s: &str) -> Result<(String, String), String> {
    let mut it = s.splitn(2, '=');
    let k = it.next().unwrap();
    let v = it
        .next()
        .ok_or_else(|| format!("`{}` is not a name=value pair", s))?;
    Ok((k.to_string(), v.to_string()))
}

pub fn export(args: &Vec<String>) {
    let matches = export_command().get_matches_from(args);

    let pairs: HashMap<_, _> = matches
        .get_many::<(String, String)>("pairs")
        .unwrap_or_default()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    println!("{:?}", pairs);
}

pub fn export_command() -> Command {
    Command::new("export")
        .about("Set the export attribute for variables")
        .arg(
            Arg::new("profile")
                .short('p')
                .long("profile")
                .value_name("PROFILE")
                .help("Profile name")
                .num_args(1),
        )
        .arg(
            Arg::new("pairs")
                .value_name("NAME=WORD")
                .help("If the name of a variable is followed by = word, then the value of that variable shall be set to word.")
                .num_args(1..)
                .value_parser(ValueParser::new(parse_kv)),
        )
}
