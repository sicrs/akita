use cli::{App, Command, Context, Flag, FlagKind};
use crate::dto::{ErrMesg, ListItem, UploadRequest, UploadResponse};
use crate::url::Url;
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::StatusCode;
use std::env::var;
use std::fs::File;
use std::io::{stdin, ErrorKind, Read, Write};
use std::path::PathBuf;
use std::process;

pub fn init() -> App<AkitaClient> {
    let db_client = AkitaClient::new();
    let app: App<AkitaClient> = App::new(db_client)
        .register_default(
            Command::new("put", None, |inner: AkitaClient, c: Context| {
                let mut content = String::new();
                if !atty::is(atty::Stream::Stdin) {
                    let stream = stdin();
                    loop {
                        let mut buf = String::new();
                        let len = stream.read_line(&mut buf).unwrap();
                        if len == 0 {
                            break;
                        }
                        content.push_str(&buf);
                    }
                } else {
                    if let Some(cont) = c.get("content") {
                        content = cont;
                    } else {
                        if c.arg.len() == 0 {
                            eprintln!("\x1b[0;31merror\x1b[0m: no file specified");
                            process::exit(1);
                        }

                        let mut file: File = handle_err(File::open(c.arg[0].as_str()));
                        handle_err(file.read_to_string(&mut content));
                    }
                }
                inner.put_doc(c.get("slug"), content);
            })
            .flag(Flag::new("slug", Some("s"), FlagKind::InputFlag, "desired slug"))
            .flag(Flag::new("content", Some("c"), FlagKind::InputFlag, "document content"))
            .set_help(
                "akita put
upload content to dogbin

USAGE:
akita put [OPTIONS] [INPUT]

OPTIONS:
-c, --content <input>       use input as content       
-s, --slug <input>          slug for uploaded document"
            )
        )
        .register(Command::new(
            "get",
            None,
            |inner: AkitaClient, c: Context| {
                if c.arg.len() == 0 {
                    eprintln!("\x1b[0;31merror\x1b[0m: no slug specified");
                    process::exit(1);
                }

                let res = inner.get_doc(c.arg[0].clone());
                if let Some(filename) = c.get("output") {
                    let mut file = File::create(filename).unwrap();
                    file.write_all(res.as_bytes()).unwrap();
                    file.flush().unwrap();
                } else {
                    println!("{}", res);
                }
            },)
            .flag(Flag::new("output", Some("o"), FlagKind::InputFlag, "output filename"))
            .set_help(
                "akita get
get a document from dogbin

USAGE:
akita get [OPTION] SLUG

OPTIONS:
-o, --output <filename>         (optional) filename to write content to"
            )
        )
        .register(Command::new(
            "auth",
            None,
            |mut inner: AkitaClient, c: Context| {
                if c.arg.len() == 0 {
                    eprintln!("\x1b[0;31merror\x1b[0m: no API key specified");
                    process::exit(1)
                } else if c.arg.len() > 1 {
                    eprintln!("\x1b[0;31merror\x1b[0m: more than one argument specified");
                    process::exit(1);
                }

                inner.conf.creds = Some(c.arg[0].clone());
                inner.conf.save();
            })
            .set_help(
                "akita auth
log in to dogbin

USAGE:
akita auth [API_KEY]"
            )
        )
        .register(Command::new(
            "ls",
            None,
            |inner: AkitaClient, _c: Context| {
                let items: Vec<ListItem> = inner.list_doc();
                for item in items {
                    println!("{}", item);
                }
            })
            .set_help(
                "akita ls
list documents uploaded
NOTE: will only work if you are logged in

USAGE:
akita list"
            )
        )
        .register(Command::new(
                "logout",
                None,
                |mut inner: AkitaClient, _c: Context| {
                    // reset credentials
                    inner.conf.creds = None;
                    inner.conf.save();
                    println!("Logged out");
                })
        );

    return app;
}

#[derive(Debug)]
pub struct Config {
    provider: Url,
    creds: Option<String>,
    handle: Option<PathBuf>,
}

impl Config {
    pub fn new() -> Config {
        Config {
            provider: Url::new(),
            creds: None,
            handle: None,
        }
    }

    pub fn get() -> Config {
        match var("HOME") {
            Ok(hpath) => {
                let confpath = PathBuf::from(hpath).join(".akita.conf");

                let confpathstr: &str = confpath.to_str().unwrap();
                match File::open(confpathstr) {
                    Ok(mut file) => {
                        let mut conf = String::new();
                        handle_err(file.read_to_string(&mut conf));

                        let mut config = Config::new();

                        for line in conf.lines() {
                            let p: Vec<&str> = line.split(" = ").collect();

                            match p[0] {
                                "provider" => {
                                    *config.provider = String::from(p[1]);
                                }
                                "creds" => {
                                    config.creds = Some(String::from(p[1]));
                                }
                                _ => {
                                    return Config {
                                        provider: Url::from_str("https://del.dog"),
                                        creds: None,
                                        handle: Some(confpath),
                                    }
                                }
                            }
                        }

                        config.handle = Some(confpath);
                        return config;
                    }
                    Err(error) => match error.kind() {
                        ErrorKind::NotFound => {
                            return Config {
                                provider: Url::from_str("https://del.dog"),
                                creds: None,
                                handle: None,
                            };
                        }
                        _ => {
                            eprintln!("\x1b[0;31merror\x1b[0m: {}", error);
                            process::exit(1);
                        }
                    },
                }
            }
            Err(err) => {
                eprintln!("\x1b[0;31merror\x1b[0m: couldn't get HOME env var: {}", err);
                process::exit(1);
            }
        }
    }

    pub fn save(self) {
        let mut confstring = String::new();
        confstring.push_str(self.provider.export().as_str());
        if let Some(cred) = &self.creds {
            confstring.push_str(format!("creds = {}\n", cred).as_str());
        }

        if let Some(path) = self.handle {
            let mut file = File::create(path.to_str().unwrap()).unwrap();
            file.write_all(confstring.as_bytes()).unwrap();
            file.flush().unwrap();
        }
    }
}

pub struct AkitaClient {
    conf: Config,
    client: Client,
}

impl AkitaClient {
    pub fn new() -> AkitaClient {
        let conf = Config::get();
        AkitaClient {
            conf,
            client: Client::new(),
        }
    }
}

impl AkitaClient {
    pub fn put_doc(&self, slug: Option<String>, content: String) {
        if content.len() == 0 {
            eprintln!("\x1b[0;31merror\x1b[0m: no content provided.");
        }

        let uri = self.conf.provider.get().join("documents");
        let mut req: RequestBuilder = self.client.post(uri.to_str().unwrap());
        if let Some(key) = slug {
            // slug is set, so use a JSON object to form request
            let dtobj = UploadRequest { slug: key, content };

            req = req
                .header("Content-Type", "application/json")
                .body(serde_json::to_string(&dtobj).unwrap());
        } else {
            // slug is not present, so use memetype text/plain
            req = req.header("Content-Type", "text/plain").body(content);
        }

        if let Some(cred) = &self.conf.creds {
            req = req.header("X-Api-Key", cred.as_str());
        }
        let response = handle_err(req.send());
        match response.status() {
            StatusCode::OK => {
                let text = handle_err(response.text());
                let b: UploadResponse = serde_json::from_str(text.as_str()).unwrap();
                println!(
                    "url: \x1b[0;32m{}/{}\x1b[0m",
                    self.conf.provider.get().to_str().unwrap(),
                    b.key
                );
            }
            _ => {
                let text = handle_err(response.text());
                let mes: ErrMesg = serde_json::from_str(text.as_str()).unwrap();
                eprintln!("\x1b[0;31merror\x1b[0m: {}", mes.message);
                process::exit(1);
            }
        }
    }

    pub fn get_doc(&self, slug: String) -> String {
        if slug == "" {
            eprintln!("\x1b[0;31merror\x1b[0m: no slug provided");
            process::exit(1);
        }

        let uri = self
            .conf
            .provider
            .get()
            .join(format!("raw/{}", slug).as_str());
        let req = self.client.get(uri.to_str().unwrap());
        let response = handle_err(req.send());
        let status = response.status();
        let text = handle_err(response.text());
        match status {
            StatusCode::OK => {
                return text;
            }
            _ => {
                eprintln!("\x1b[0;31merror\x1b[0m: {}", text);
                process::exit(1);
            }
        }
    }

    pub fn list_doc(&self) -> Vec<ListItem> {
        if let Some(key) = &self.conf.creds {
            let uri = self
                .conf
                .provider
                .get()
                .join("api/v1/docs");
            let req = self.client.get(uri.to_str().unwrap())
                .header("X-Api-Key", key.as_str());
            let response = handle_err(req.send());
            let status = response.status();
            let text = handle_err(response.text());
            match status {
                StatusCode::OK => {
                    let list: Vec<ListItem> = serde_json::from_str(text.as_str()).unwrap();
                    return list;
                },
                _ => {
                    eprintln!("\x1b[0;31merror\x1b[0m: {}", text);
                    process::exit(1);
                }
            }

        } else {
            eprintln!("\x1b[0;31merror\x1b[0m: no API key provided; use `akita auth`");
            process::exit(1);
        } 
    }
}

fn handle_err<T, E>(res: Result<T, E>) -> T
where
    E: std::error::Error,
{
    match res {
        Ok(val) => return val,
        Err(err) => {
            eprintln!("\x1b[0;31merror\x1b[0m: {}", err);
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod test {
    use super::{AkitaClient};
    fn get_test() {
        let a = AkitaClient::new();
        let res = a.get_doc("changelog".to_string());
        println!("{}", res);
    }
}
