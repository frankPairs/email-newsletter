use config::{Config, ConfigError, File};
use secrecy::{ExposeSecret, Secret};
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::{
    postgres::{PgConnectOptions, PgSslMode},
    ConnectOptions,
};

use crate::domain::subscriber_email::SubscriberEmail;

#[derive(Debug)]
pub enum Environment {
    Development,
    Production,
}

#[derive(serde::Deserialize, Clone)]
pub struct Settings {
    pub application: ApplicationSettings,
    pub database: DatabaseSettings,
    pub email_client: EmailClientSettings,
    pub redis: RedisSettings,
}

#[derive(serde::Deserialize, Clone)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub base_url: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    pub api_key: Secret<String>,
}

#[derive(serde::Deserialize, Clone)]
pub struct DatabaseSettings {
    pub username: String,
    // secrecy protects secret information and prevents them to be exposed (eg: via logs)
    pub password: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub name: String,
    pub require_ssl: bool,
}

#[derive(serde::Deserialize, Clone)]
pub struct RedisSettings {
    pub port: u16,
    pub host: String,
}

impl Settings {
    pub fn get_address(&self) -> String {
        format!(
            "{}:{}",
            self.application.get_host(),
            self.application.get_port()
        )
    }

    pub fn get_app_base_url(&self) -> String {
        self.application.get_base_url()
    }

    pub fn get_db_options(&self) -> PgConnectOptions {
        self.database.get_db_options()
    }

    pub fn get_email_client_sender(&self) -> Result<SubscriberEmail, String> {
        return self.email_client.get_sender_email();
    }

    pub fn get_email_client_base_url(&self) -> String {
        return self.email_client.get_base_url();
    }

    pub fn get_email_client_api(&self) -> Secret<String> {
        return self.email_client.get_api_key();
    }

    pub fn set_email_client_base_url(&mut self, new_base_url: String) {
        self.email_client.base_url = new_base_url
    }

    pub fn get_db_name(&self) -> String {
        self.database.get_name()
    }

    pub fn get_db_username(&self) -> String {
        self.database.get_username()
    }

    pub fn set_db_name(&mut self, db_name: String) {
        self.database.set_name(db_name)
    }

    pub fn set_app_port(&mut self, port: u16) {
        self.application.port = port;
    }

    pub fn get_redis_address(&self) -> String {
        self.redis.get_address()
    }
}

impl DatabaseSettings {
    pub fn get_db_options(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };

        let mut db_options = PgConnectOptions::new()
            .host(&self.host)
            .password(&self.password.expose_secret())
            .username(&self.username)
            .port(self.port)
            .database(&self.name)
            .ssl_mode(ssl_mode);

        db_options.log_statements(tracing::log::LevelFilter::Trace);

        db_options
    }

    pub fn get_uri(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.name
        )
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_username(&self) -> String {
        self.username.clone()
    }

    pub fn set_name(&mut self, new_db_name: String) {
        self.name = new_db_name
    }
}

impl ApplicationSettings {
    pub fn get_port(&self) -> u16 {
        self.port
    }

    pub fn get_host(&self) -> String {
        self.host.clone()
    }

    pub fn get_base_url(&self) -> String {
        self.base_url.clone()
    }
}

impl EmailClientSettings {
    pub fn get_sender_email(&self) -> Result<SubscriberEmail, String> {
        SubscriberEmail::parse(self.sender_email.clone())
    }

    pub fn get_base_url(&self) -> String {
        self.base_url.clone()
    }

    pub fn get_api_key(&self) -> Secret<String> {
        self.api_key.clone()
    }

    pub fn set_base_url(&mut self, new_base_url: String) {
        self.base_url = new_base_url
    }
}

impl RedisSettings {
    pub fn get_address(&self) -> String {
        format!("redis://{}:{}", self.host, self.port)
    }
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Development => "development",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "development" => Ok(Self::Development),
            "production" => Ok(Self::Production),
            unknown_env => Err(format!(
                "{} is not supported environment. Use either 'development' or 'production'.",
                unknown_env
            )),
        }
    }
}

pub fn get_configuration() -> Result<Settings, ConfigError> {
    let root_path = std::env::current_dir().expect("Failed to determine the current directory");
    let config_directory = root_path.join("config");
    // Uses development environment by default
    let enviroment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "development".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT");
    let config_base_filepath = config_directory.join("base");
    let config_env_filepath = config_directory.join(enviroment.as_str());

    // It merges the base configuration file with the one from the specific environment (development or production)
    let settings = Config::builder()
        .add_source(File::from(config_base_filepath).required(true))
        .add_source(File::from(config_env_filepath).required(true))
        // Merge settings from environment variables with a prefix of APP and "__" separator
        // E.g APP_APPLICATION__PORT would set Settings.application.port
        .add_source(config::Environment::with_prefix("app").separator("__"))
        .build()?;

    tracing::info!("Application environment = {:?}", enviroment);

    // Try to convert the value from the configuration file into a Settings type
    settings.try_deserialize()
}
