mod sidecar;

use serde::{Deserialize, Serialize};
pub use sidecar::recognize;

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
