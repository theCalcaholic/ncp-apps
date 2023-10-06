use serde::Deserialize;
pub mod containers;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NcpConfigParamType {
    String,
    Password,
    Bool,
    Multiline
}

impl NcpConfigParamType {
    fn default() -> Self { NcpConfigParamType::String }
}

#[derive(Debug, Deserialize)]
pub struct NcpConfigParam {
    id: String,
    name: String,
    value: String,
    suggest: String,
    #[serde(rename = "type", default = "NcpConfigParamType::default")]
    param_type: NcpConfigParamType,
}
#[derive(Debug, Deserialize)]
pub struct NcpAppConfig {
    id: String,
    name: String,
    title: String,
    description: String,
    info: String,
    infotitle: String,
    params: Vec<NcpConfigParam>
}
