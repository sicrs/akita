use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Debug)]
pub struct UploadRequest {
    pub slug: String,
    pub content: String,
}

#[derive(Deserialize, Debug)]
pub struct UploadResponse {
    #[serde(rename = "isUrl", default)]
    pub is_url: bool,
    pub key: String,
}

#[derive(Deserialize)]
pub struct ErrMesg {
    pub message: String,
}

#[derive(Deserialize)]
pub struct Document {
    #[serde(rename = "_url")]
    pub slug: String,
    pub is_url: bool,
    pub content: String,
    pub viewcount: usize,
}

#[derive(Debug, Deserialize)]
pub struct ListItem {
    pub slug: String,
    pub created: String,
    #[serde(rename = "type")]
    pub doctype: String,
}

impl fmt::Display for ListItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (\x1b[0;33m{}\x1b[0m)\ncreated: {}\n",
            self.slug,
            self.doctype,
            self.created,
        )
    }
}