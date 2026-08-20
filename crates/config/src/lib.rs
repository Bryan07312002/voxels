use std::{fs, path::Path};

use core_types::ViewDistance;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_max_view_distance")]
    pub max_view_distance: ViewDistance,
}

fn default_max_view_distance() -> ViewDistance {
    ViewDistance(12)
}

fn default_host() -> String {
    String::from("0.0.0.0")
}

fn default_port() -> u16 {
    8080
}

#[derive(Deserialize, Debug)]
pub struct ClientConfig {
    #[serde(default = "default_view_distance")]
    pub view_distance: ViewDistance,

    #[serde(default = "default_wireframe")]
    pub wireframe: bool,
}

fn default_view_distance() -> ViewDistance {
    ViewDistance(8)
}

fn default_wireframe() -> bool {
    true
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub server: ServerConfig,
    pub client: ClientConfig,
}

pub fn load_server_config(path: &Path) -> Result<ServerConfig, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config.server)
}

pub fn load_client_config(path: &Path) -> Result<ClientConfig, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config.client)
}

pub fn load_config(path: &Path) -> Result<Config, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}
