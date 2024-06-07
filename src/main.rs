#[allow(unused_imports)]
use std::io::{self, Write};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};

use chumsky::{
    error::Cheap,
    extra::Err,
    primitive::{choice, just},
    text::{digits, whitespace},
    IterParser, Parser,
};

fn search_path(bin: &str) -> Option<PathBuf> {
    std::env::var("PATH")
        .unwrap_or_else(|_| String::new())
        .split(':')
        .map(Path::new)
        .filter(|p| p.is_dir())
        .find_map(|dir| {
            dir.read_dir()
                .expect("to be a directory")
                .into_iter()
                .filter_map(Result::ok)
                .find_map(|entry| {
                    if entry.file_name() == bin {
                        Some(entry.path())
                    } else {
                        None
                    }
                })
        })
}

#[derive(Debug, Clone)]
enum ShellCmd<'a> {
    Builtin(Builtin),
    #[allow(unused)]
    Unknown {
        cmd: &'a str,
        args: Vec<String>,
    },
}

impl<'a> ShellCmd<'a> {
    pub fn parser() -> impl Parser<'a, &'a str, ShellCmd<'a>, Err<Cheap>> {
        let builtin = Builtin::parser().map(ShellCmd::Builtin);

        // TODO: Better shell splitting w/ strings, escapes etc.
        let unknown = chumsky::primitive::none_of(" \t\r\n")
            .repeated()
            .to_slice()
            .separated_by(whitespace().at_least(1).ignored())
            .allow_leading()
            .allow_trailing()
            .collect()
            .map(|mut args: Vec<&str>| {
                let mut drain = args.drain(..);
                ShellCmd::Unknown {
                    cmd: drain.next().expect("command"),
                    args: drain.map(str::to_owned).collect(),
                }
            });

        choice((builtin, unknown))
    }
}

#[derive(Debug, Clone)]
enum Builtin {
    Exit { code: i32 },
    Echo { text: Vec<String> },
    Type { cmd: String },
    Pwd,
    Cd { dir: String },
}

impl Builtin {
    pub fn parser<'a>() -> impl Parser<'a, &'a str, Builtin, Err<Cheap>> {
        choice((
            just("exit").then(whitespace().ignored()).ignore_then(
                digits(10)
                    .to_slice()
                    .or_not()
                    .map(|digits: Option<&str>| {
                        digits.map(|digits| digits.parse::<i32>().expect("to have been validated"))
                    })
                    .map(|code| Builtin::Exit {
                        code: code.unwrap_or(0),
                    }),
            ),
            just("echo").ignore_then(
                chumsky::primitive::none_of(" \t\r\n")
                    .repeated()
                    .collect()
                    .separated_by(whitespace().at_least(1).ignored())
                    .allow_leading()
                    .allow_trailing()
                    .collect()
                    .map(|args| Builtin::Echo { text: args }),
            ),
            just("type").ignore_then(whitespace()).ignore_then(
                chumsky::primitive::none_of(" \t\r\n")
                    .repeated()
                    .collect()
                    .map(|s| Builtin::Type { cmd: s }),
            ),
            just("pwd").to(Builtin::Pwd),
            just("cd").ignore_then(whitespace()).ignore_then(
                chumsky::primitive::none_of(" \t\r\n")
                    .repeated()
                    .collect()
                    .map(|s| Builtin::Cd { dir: s }),
            ),
        ))
    }
}

fn main() -> eyre::Result<()> {
    let mut input = String::new();

    'mainloop: loop {
        // Print the prompt
        print!("$ ");
        io::stdout().flush()?;

        input.clear();

        // Wait for user input
        let stdin = io::stdin();
        stdin.read_line(&mut input)?;

        if input.is_empty() {
            continue;
        }

        let parsed = ShellCmd::parser().parse(input.trim());

        if parsed.has_errors() {
            for e in parsed.errors() {
                println!("{e}");
            }
            // return Err(Report::);
        }

        match parsed.output().expect("to have output") {
            ShellCmd::Builtin(builtin) => match builtin {
                Builtin::Exit { code } => {
                    if *code == 0 {
                        break 'mainloop;
                    }
                    return Err(std::io::Error::from_raw_os_error(*code).into());
                }
                Builtin::Echo { text } => {
                    text.iter().enumerate().for_each(|(i, s)| {
                        print!("{s}");
                        if i < text.len() - 1 {
                            print!(" ");
                        }
                    });
                    println!("");
                }
                Builtin::Type { cmd } => {
                    let Ok(parsed) = ShellCmd::parser().parse(&*cmd).into_result() else {
                        break 'mainloop;
                    };
                    match parsed {
                        ShellCmd::Builtin(_) => println!("{cmd} is a shell builtin"),
                        ShellCmd::Unknown { cmd, args: _ } => {
                            if let Some(bin) = search_path(cmd) {
                                println!("{} is {}", cmd, bin.display());
                            } else {
                                println!("{cmd} not found");
                            }
                        }
                    }
                }
                Builtin::Pwd => {
                    println!(
                        "{}",
                        std::env::current_dir().expect("current dir").display()
                    );
                }
                Builtin::Cd { dir } => {
                    let dir = Path::new(dir);
                    if dir.is_dir() {
                        std::env::set_current_dir(dir)?;
                    // } else if std::env::current_dir()?.join(dir).is_dir() {
                    //     std::env::set_current_dir(std::env::current_dir()?.join(dir))?;
                    } else {
                        println!("{}: No such file or directory", dir.display());
                    }
                }
            },
            ShellCmd::Unknown { cmd, args } => {
                if let Some(bin) = search_path(cmd) {
                    std::process::Command::new(bin)
                        .args(args)
                        .stdin(Stdio::inherit())
                        .stdout(Stdio::inherit())
                        .stderr(Stdio::inherit())
                        .spawn()
                        .expect("to spawn program")
                        .wait()
                        .expect("to wait for process");
                } else {
                    println!("{cmd}: command not found");
                }
            }
        }
    }

    Ok(())
}
