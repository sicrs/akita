use std::collections::HashMap;
use std::process;

#[derive(Debug)]
pub struct Context {
    pub arg: Vec<String>,
    pub flagmap: HashMap<&'static str, FlagRes>,
}

#[derive(Debug)]
pub enum FlagRes {
    Input(String),
    Opt,
}

impl FlagRes {
    pub fn val(&self) -> String {
        if let FlagRes::Input(value) = self {
            return value.clone();
        } else {
            panic!("FlagRes: tried to get value from an option flag");
        }
    }
}

impl Context {
    pub fn new() -> Context {
        Context {
            arg: Vec::new(),
            flagmap: HashMap::new(),
        }
    }

    pub fn is_set(&self, ident: &str) -> bool {
        return self.flagmap.contains_key(ident);
    }

    pub fn get(&self, ident: &str) -> Option<String> {
        if let Some(value) = self.flagmap.get(ident) {
            if let FlagRes::Input(content) = value {
                return Some(content.to_string());
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
}

pub struct App<T> {
    cmds: Vec<Command<T>>,
    pub inner: T,
}

impl<T> App<T> {
    pub fn new(inner: T) -> App<T> {
        App {
            cmds: Vec::new(),
            inner,
        }
    }

    pub fn register(mut self, cmd: Command<T>) -> App<T> {
        self.cmds.push(cmd);
        return self;
    }

    pub fn run(self, arg: Vec<String>) {
        if arg.len() == 0 {
            eprintln!("command not specified; try using --help");
            process::exit(1);
        }
        // build context
        // get command used
        let cmd: Command<T>;

        {
            let mut cmnd: Option<Command<T>> = None;
            for command in self.cmds.into_iter() {
                if arg[0] == command.ident || arg[0] == command.alias {
                    cmnd = Some(command);
                    break;
                }
            }

            if let Some(command) = cmnd {
                cmd = command;
            } else {
                eprint!("no command {} found", arg[0]);
                process::exit(1);
            }
        }
        let mut ctx = Context::new();

        let mut count = 1;
        loop {
            if count > arg.len() - 1 {
                break;
            }

            if arg[count].starts_with("-") {
                if arg[count].starts_with("--") {
                    // does not use abbreviated flags
                    // iterate over given flags and terminate loop after match
                    for flag in &cmd.flags {
                        if arg[count].contains(flag.alias) {
                            // match
                            if flag.kind == FlagKind::InputFlag {
                                if count + 1 == arg.len() || arg[count + 1].starts_with("-") {
                                    eprintln!("No argument for flag {} found", arg[count]);
                                    process::exit(1);
                                }

                                if ctx.flagmap.contains_key(flag.ident) {
                                    eprintln!(
                                        "error: the flag -{} has already been specified before",
                                        flag.alias
                                    );
                                    process::exit(1);
                                }

                                ctx.flagmap
                                    .insert(flag.ident, FlagRes::Input(arg[count + 1].clone()));
                                count += 1;
                            } else {
                                ctx.flagmap.insert(flag.ident, FlagRes::Opt);
                            }
                            break;
                        }
                    }
                } else {
                    // uses abbreviated flags
                    // iterate over all flags in the specified command
                    for flag in &cmd.flags {
                        if arg[count].contains(flag.ident) {
                            if flag.kind == FlagKind::InputFlag {
                                if arg[count] == format!("-{}", flag.ident) {
                                    if count + 1 == arg.len() || arg[count + 1].starts_with("-") {
                                        eprintln!("No argument for flag {} found", arg[count]);
                                        process::exit(1);
                                    }
                                    if ctx.flagmap.contains_key(flag.ident) {
                                        eprintln!(
                                            "error: the flag -{} has already been specified before",
                                            flag.ident
                                        );
                                        process::exit(1);
                                    }

                                    ctx.flagmap
                                        .insert(flag.ident, FlagRes::Input(arg[count + 1].clone()));
                                    count += 1;
                                    break;
                                } else {
                                    eprintln!(
                                        "You can't put an input flag alongside an option flag"
                                    );
                                    process::exit(1);
                                }
                            } else {
                                ctx.flagmap.insert(flag.ident, FlagRes::Opt);
                            }
                        }
                    }
                }
            } else {
                // does not contain a flag, considered input
                ctx.arg.push(arg[count].clone());
            }

            // move to next index
            count += 1;
        }

        // execute command
        (cmd.directive)(self.inner, ctx);
    }
}

pub struct Command<T> {
    ident: &'static str,
    alias: &'static str,
    description: String,
    directive: Box<dyn Fn(T, Context) + 'static>,
    flags: Vec<Flag>,
}

impl<U> Command<U> {
    pub fn new<T>(ident: &'static str, alias: &'static str, desc: &str, directive: T) -> Command<U>
    where
        T: Fn(U, Context) + 'static,
    {
        Command {
            ident,
            alias,
            directive: Box::new(directive),
            description: String::from(desc),
            flags: Vec::new(),
        }
    }

    pub fn flag(mut self, f: Flag) -> Command<U> {
        self.flags.push(f);
        return self;
    }
}

#[derive(PartialEq)]
pub enum FlagKind {
    InputFlag,
    OptFlag,
}

pub struct Flag {
    alias: &'static str,
    description: String,
    ident: &'static str,
    kind: FlagKind,
}

impl Flag {
    pub fn new(ident: &'static str, alias: &'static str, kind: FlagKind) -> Flag {
        Flag {
            alias,
            description: String::new(),
            ident,
            kind,
        }
    }

    pub fn description(mut self, desc: &str) -> Flag {
        self.description = String::from(desc);
        return self;
    }
}
#[cfg(test)]
mod tests {
    use super::{Command, Context, Flag, FlagKind, App};
    #[test]
    fn interpreter_test() {
        let mut intr = App::new().register(
            Command::new(
                "test",
                "t",
                |c: Context| {
                    println!("{:?}", c);
                },
            )
            .flag(Flag::new("o", "output", FlagKind::InputFlag))
            .flag(Flag::new("i", "input", FlagKind::InputFlag)),
        );

        let args: Vec<String> = vec![
            "test".to_string(),
            "--output".to_string(),
            "flag_input".to_string(),
            "--input".to_string(),
            "input_two".to_string(),
            "input_three".to_string(),
            "input_four".to_string(),
        ];
        intr.run(args);
    }
}
