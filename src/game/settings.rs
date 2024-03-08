use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Multiplier(f64);

impl Multiplier {
    pub fn to_string(&self) -> String {
        format!("{:.2}", self.0)
    }
}

impl Default for Multiplier {
    fn default() -> Self {
        Self(1.0)
    }
}
