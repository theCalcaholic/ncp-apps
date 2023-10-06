use serde::{Deserialize, Serialize, Serializer};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterValue {
    String(String),
    Boolean(bool),
    Password(String),
    Multiline(String),

}
impl Default for ParameterValue {
    fn default() -> Self {
        Self::String("".into())
    }
}

impl Serialize for ParameterValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        seria
    }

}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Parameters {
    id: String,
    name: String,
    value: ParameterValue
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Config {
    parameters: Parameters

}

pub struct Context {

}

pub struct ContextRequest {

}
