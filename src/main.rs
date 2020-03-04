use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::process;

use std::os::unix::process::CommandExt;

extern crate toml;

type EnvMap = HashMap<String, String>;

type EnvMapOrError = Result<EnvMap, String>;

fn print_usage() {
    eprintln!("Usage: envf [(-f FILE) ...] [-s] COMMAND ...");
    eprintln!("");
    eprintln!("Run COMMAND in an environment augmented with the variables listed in each FILE.");
    eprintln!("");
    eprintln!("Options:");
    eprintln!("  -f FILE     Add values read from FILE to the environment in which COMMAND is run.");
    eprintln!("              FILE is a TOML (https://github.com/toml-lang/toml) table of scalar values.");
    eprintln!("  -s          Silence warnings about unprocessable files.");
    eprintln!("  -h, --help  Display this message.");
    eprintln!("");
    eprintln!("Source: https://github.com/thilp/envf");
}

fn error_without_usage(msg: &str) -> ! {
    eprintln!("ERROR: {}", msg);
    process::exit(1);
}

fn error_with_usage(msg: &str) -> ! {
    eprintln!("ERROR: {}", msg);
    eprintln!("");
    print_usage();
    process::exit(1);
}

fn warning(msg: &str) {
    eprintln!("WARNING: {}", msg);
}

struct Config {
    files: Vec<String>,
    silent: bool,
    command: Vec<String>,
}

fn main() {
    let config = match parse_args(env::args().skip(1)) {
        ArgParseResult::Help => {
            print_usage();
            process::exit(0);
        },
        ArgParseResult::Err(s) => error_with_usage(s),
        ArgParseResult::Config(c) => c,
    };
    let mut map = EnvMap::new();
    for path in config.files {
        match read_env_file(&path) {
            Err(msg) => {
                if !config.silent {
                    warning(&format!("{} ignored: {}", path, msg));
                }
            }
            Ok(m) => {
                for (k, v) in m {
                    map.insert(k, v);
                }
            }
        }
    }
    let err = process::Command::new(&config.command[0])
        .args(config.command.iter().skip(1).collect::<Vec<&String>>())
        .envs(&map)
        .exec();
    error_without_usage(&format!(
        "Couldn't execute command {:?}: {}",
        config.command, err
    ));
}

enum ArgParseResult {
    Config(Config),
    Err(&'static str),
    Help,
}

fn parse_args(args: impl Iterator<Item = String>) -> ArgParseResult {
    let mut files: Vec<String> = vec![];
    let mut silent = false;
    let mut args = args.peekable();
    loop {
        match args.peek() {
            None => break,
            Some(arg) => {
                if arg == "-h" || arg == "--help" {
                    return ArgParseResult::Help;
                } else if arg == "-s" {
                    silent = true;
                } else if arg == "-f" {
                    args.next();
                    match args.peek() {
                        None => return ArgParseResult::Err("Trailing -f"),
                        Some(path) => files.push(path.to_string()),
                    }
                } else if arg.starts_with("-f=") {
                    files.push(arg[3..].to_string());
                } else {
                    break;
                }
            }
        }
        args.next();
    }
    let cmd: Vec<String> = args.collect();
    if cmd.len() == 0 {
        ArgParseResult::Err("No command to execute was provided.")
    } else {
        ArgParseResult::Config(Config {
            files: files,
            silent: silent,
            command: cmd,
        })
    }
}

fn read_env_file(path: &str) -> EnvMapOrError {
    match fs::read_to_string(path) {
        Err(err) => Err(format!("Could not read contents: {}", err.description())),
        Ok(body) => match body.parse::<toml::Value>() {
            Err(err) => Err(format!("Invalid TOML: {}", err)),
            Ok(doc) => match doc.try_into::<toml::value::Table>() {
                Err(err) => Err(format!("Unexpected format: {}", err.description())),
                Ok(table) => table_into_env_map(&table),
            },
        },
    }
}

fn table_into_env_map(table: &toml::value::Table) -> EnvMapOrError {
    table.iter().fold(Ok(EnvMap::new()), add_field)
}

fn add_field(z: EnvMapOrError, (k, v): (&String, &toml::Value)) -> EnvMapOrError {
    match z {
        Err(_) => z,
        Ok(m) => match stringify(v) {
            Some(s) => {
                let mut n = m.clone();
                n.insert(String::from(k), s);
                Ok(n)
            }
            None => Err(format!(
                "value for {} ({:?}) can't be converted into a string",
                k, v
            )),
        },
    }
}

fn stringify(v: &toml::Value) -> Option<String> {
    match v {
        toml::value::Value::String(s) => Some(String::from(s)),
        toml::value::Value::Integer(x) => Some(format!("{}", x)),
        toml::value::Value::Float(x) => Some(format!("{}", x)),
        toml::value::Value::Boolean(x) => Some(format!("{}", x)),
        toml::value::Value::Datetime(x) => Some(format!("{}", x)),
        _ => None,
    }
}
