use crate::KvError;
use serde::{Deserialize, Serialize};
use std::fs;

/// 你可以看到，在 Rust 下，有了 serde 的帮助，
/// 处理任何已知格式的配置文件，是多么容易的一件事情。
/// 我们只需要定义数据结构，并为数据结构使用 Serialize/Deserialize 派生宏，
/// 就可以处理任何支持 serde 的数据结构。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ServerConfig {
    pub general: GeneralConfig,
    pub storage: StorageConfig,
    pub tls: ServerTlsConfig,
    pub log: LogConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClientConfig {
    pub general: GeneralConfig,
    pub tls: ClientTlsConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GeneralConfig {
    pub addr: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LogConfig {
    pub path: String,
    pub rotation: RotationConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum RotationConfig {
    Hourly,
    Daily,
    Never,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "args")]
pub enum StorageConfig {
    MemTable,
    SledDb(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ServerTlsConfig {
    pub cert: String,
    pub key: String,
    pub ca: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ClientTlsConfig {
    pub domain: String,
    pub identity: Option<(String, String)>,
    pub ca: Option<String>,
}

impl ServerConfig {
    pub fn load(path: &str) -> Result<Self, KvError> {
        let config = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&config)?;
        Ok(config)
    }
}

impl ClientConfig {
    pub fn load(path: &str) -> Result<Self, KvError> {
        let config = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&config)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_config_should_be_loaded() {
        let result: Result<ServerConfig, toml::de::Error> =
            toml::from_str(include_str!("../fixtures/server.conf"));
        assert!(result.is_ok());
    }

    #[test]
    fn client_config_should_be_loaded() {
        let result: Result<ClientConfig, toml::de::Error> =
            toml::from_str(include_str!("../fixtures/client.conf"));
        assert!(result.is_ok());
    }
}
