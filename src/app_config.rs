use config::{Config, ConfigError};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub address: String,
}
#[derive(Debug, Deserialize)]
pub struct Modbus {
    pub address: String,
    pub slave_id: u8,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct ModbusConfig {
    pub configs: Vec<Modbus>,
}
#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub modbus: ModbusConfig,
}

pub fn load_config() -> Result<AppConfig, ConfigError> {
    let settings = Config::builder()
        // Add in `./config.toml`
        .add_source(config::File::with_name("./config"))
        // Add in settings from the environment (with a prefix of APP)
        // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
        // .add_source(config::Environment::with_prefix("APP"))
        .build()
        .unwrap();
    Ok(settings.try_deserialize::<AppConfig>().unwrap())
}
