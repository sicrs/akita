use serde::{Deserialize, Serialize};

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