use config::{Config, ConfigError, File};
use secrecy::{ExposeSecret, Secret};

pub enum Environment {
    Development,
    Production,
}

#[derive(serde::Deserialize)]
pub struct Settings {
    pub application: ApplicationSettings,
    pub database: DatabaseSettings,
}

#[derive(serde::Deserialize)]
pub struct ApplicationSettings {
    pub port: u16,
    pub host: String,
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    // secrecy protects secret information and prevents them to be exposed (eg: via logs)
    pub password: Secret<String>,
    pub port: u16,
    pub host: String,
    pub db_name: String,
}

impl Settings {
    pub fn get_address(&self) -> String {
        format!(
            "{}:{}",
            self.application.get_host(),
            self.application.get_port()
        )
    }

    pub fn get_db_url(&self) -> Secret<String> {
        self.database.get_url()
    }

    pub fn get_db_url_without_name(&self) -> Secret<String> {
        self.database.get_url_without_db_name()
    }

    pub fn set_db_name(&mut self, db_name: String) {
        self.database.set_db_name(db_name)
    }
}

impl DatabaseSettings {
    pub fn get_url(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.db_name
        ))
    }

    pub fn get_url_without_db_name(&self) -> Secret<String> {
        Secret::new(format!(
            "postgres://{}:{}@{}:{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port
        ))
    }

    pub fn set_db_name(&mut self, new_db_name: String) {
        self.db_name = new_db_name
    }
}

impl ApplicationSettings {
    pub fn get_port(&self) -> u16 {
        self.port
    }

    pub fn get_host(&self) -> String {
        self.host.clone()
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
            "production" => Ok(Self::Development),
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
        .build()?;

    // Try to convert the value from the configuration file into a Settings type
    settings.try_deserialize()
}
