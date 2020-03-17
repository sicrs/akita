use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

#[derive(Debug)]
pub struct Url {
    https: bool,
    base_url: String,
    is_dogbin: bool,
}

impl Url {
    pub fn new() -> Url {
        Url {
            https: true,
            base_url: String::new(),
            is_dogbin: true,
        }
    }

    pub fn from_str(base_url: &str) -> Url {
        // https is true by default
        Url {
            https: base_url.contains("https"),
            base_url: String::from(base_url),
            is_dogbin: true,
        }
    }

    pub fn get(&self) -> PathBuf {
        let pref: &str = if self.https { "http" } else { "https" };

        if self.base_url.contains("http") {
            return PathBuf::from(self.base_url.as_str());
        } else {
            return PathBuf::from(format!("{}://{}", pref, self.base_url).as_str());
        }
    }

    pub fn set_http(mut self) -> Url {
        self.https = false;
        self
    }

    pub fn export(&self) -> String {
        return format!("provider = {}\n", self.base_url.as_str());
    }
}

impl Deref for Url {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.base_url
    }
}

impl DerefMut for Url {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base_url
    }
}
