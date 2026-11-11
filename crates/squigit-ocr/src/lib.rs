pub mod models;
mod network;
pub mod ocr;
pub mod sidecar;

pub use ocr::recognize;
use serde::{Deserialize, Serialize};

pub enum Recognition {
    Text(Vec<TextBox>),
    Cancelled,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TextBox {
    pub text: String,
    pub confidence: f64,
    pub r#box: [[f64; 2]; 4],
}
