use cli::{App, Command, Context, Flag, FlagKind};
use crate::dto::{Document, ErrMesg, UploadRequest, UploadResponse};
use crate::url::Url;
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::StatusCode;
use std::env::{var, Args};
use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::path::PathBuf;
use std::process;

pub fn init() -> App<AkitaClient> {
    let db_client = AkitaClient::new();
    let app: App<AkitaClient> = App::new(db_client)
        .register(
            Command::new("put", "p", "", |inner: AkitaClient, c: Context| {
                if let Some(content) = c.get("c") {
                    // content flag is set
                    inner.put_doc(c.get("s"), content)
                } else {
                    // a file is specified
                    if c.arg.len() == 0 {
                        eprintln!("error: no file specified");
                        process::exit(1);
                    }
                    
                    let mut content = String::new();
                    let mut file: File = handle_err(File::open(c.arg[0].as_str()));
                    handle_err(file.read_to_string(&mut content));

                    inner.put_doc(c.get("s"), content);
                }
            })
            .flag(Flag::new("s", "slug", FlagKind::InputFlag, "desired slug"))
            .flag(Flag::new("c", "content", FlagKind::InputFlag, "document content")),
        )
        .register(Command::new(
            "get",
            "g",
            "",
            |inner: AkitaClient, c: Context| {
                if c.arg.len() == 0 {
                    eprintln!("error: no slug specified");
                    process::exit(1);
                }

                let res = inner.get_doc(c.arg[0].clone());
                println!("{}", res);
            },
        ));

    return app;
}

#[derive(Debug)]
pub struct Credentials {
    pub user: String,
    pub api_key: String,
}

impl Credentials {
    fn new(user: &str, api_key: &str) -> Credentials {
        Credentials {
            user: String::from(user),
            api_key: String::from(api_key),
        }
    }
}

#[derive(Debug)]
pub struct Config {
    provider: Url,
    creds: Option<Credentials>,
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
                                    let o: Vec<&str> = p[1].split(";").collect();
                                    config.creds = Some(Credentials::new(o[0], o[1]));
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
                            eprintln!("[Config::get] {}", error);
                            process::exit(1);
                        }
                    },
                }
            }
            Err(err) => {
                eprintln!("Couldn't get HOME env var: {}", err);
                process::exit(1);
            }
        }
    }

    pub fn save(self) {
        let mut confstring = String::new();
        confstring.push_str(self.provider.export().as_str());
        if let Some(cred) = &self.creds {
            confstring.push_str(format!("creds = {};{}\n", cred.user, cred.api_key).as_str());
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
            eprintln!("No content provided.");
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
            req = req.header("X-Api-Key", cred.api_key.as_str());
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
                eprintln!("error: {}", mes.message);
                process::exit(1);
            }
        }
    }

    pub fn get_doc(&self, slug: String) -> String {
        if slug == "" {
            eprintln!("error: no slug provided");
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
                eprintln!("error: {}", text);
                process::exit(1);
            }
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
            eprintln!("{}", err);
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod test {
    use super::{AkitaClient, Config};
    #[test]
    fn read_conf() {
        let conf = Config::get();
        println!("{:?}", conf);
        conf.save();
    }
    #[test]
    fn upload_test() {
        let a = AkitaClient::new();
        a.put_doc(None, String::from("yus"));
    }

    #[test]
    fn get_test() {
        let a = AkitaClient::new();
        let res = a.get_doc("changelog");
        println!("{}", res);
    }
}
