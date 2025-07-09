use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub server: Server,
    pub database: Database,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Server {
    pub ip: String,
    pub port: u16,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Database {
    pub ip: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
}

pub fn load() -> Result<Config, toml::de::Error> {
    let config_content = include_str!("../../maden.toml");
    toml::from_str(config_content)
}
