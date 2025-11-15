use config::{Config, ConfigError, Environment};
use serde::{Deserialize, Serialize};

use crate::config::{DatabaseConfig, HooksConfig, LogConfig, RulesConfig};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Settings {
    #[serde(default)]
    pub database: DatabaseConfig,

    #[serde(default)]
    pub logs: LogConfig,

    #[serde(default)]
    pub rules: RulesConfig,

    #[serde(default)]
    pub hooks: HooksConfig,
}

fn get_env_file_name() -> String {
    if let Ok(lef_env) = std::env::var("LEAF_ENV_FILE") {
        return lef_env;
    }
    if let Ok(lef_env) = std::env::var("LEAF_ENV") {
        match lef_env.as_str().to_lowercase().as_str() {
            "dev" => return ".env.dev".to_string(),
            "test" => return ".env.test".to_string(),
            "prod" => return ".env".to_string(),
            _ => return ".env".to_string(),
        }
    }
    return ".env".to_string();
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        // Load .env file using `LEAF_ENV` env var
        dotenvy::from_filename(get_env_file_name()).ok();

        let settings = Config::builder()
            // Load environment variables with LEAF prefix
            // The key is to use prefix_separator to handle nested structs
            .add_source(
                Environment::with_prefix("LEAF")
                    .prefix_separator("__")
                    .separator("__"),
            )
            .build()?;

        let settings: Settings = settings.try_deserialize()?;

        Ok(settings)
    }

    pub fn print_config(&self) {
        match serde_json::to_string_pretty(self) {
            Ok(json) => println!("{}", json),
            Err(err) => eprintln!("Failed to serialize settings: {}", err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    #[serial]
    fn test_env_file_var_set() {
        unsafe {
            env::set_var("LEAF_ENV_FILE", ".env.override");
        }
        assert_eq!(get_env_file_name(), ".env.override");
        unsafe {
            env::remove_var("LEAF_ENV_FILE");
        }
    }

    #[test]
    #[serial]
    fn test_leaf_env_dev() {
        unsafe {
            env::set_var("LEAF_ENV", "dev");
        }
        assert_eq!(get_env_file_name(), ".env.dev");
        unsafe {
            env::remove_var("LEAF_ENV");
        }
    }

    #[test]
    #[serial]
    fn test_leaf_env_test() {
        unsafe {
            env::set_var("LEAF_ENV", "test");
        }
        assert_eq!(get_env_file_name(), ".env.test");
        unsafe {
            env::remove_var("LEAF_ENV");
        }
    }

    #[test]
    #[serial]
    fn test_leaf_env_prod() {
        unsafe {
            env::set_var("LEAF_ENV", "prod");
        }
        assert_eq!(get_env_file_name(), ".env");
        unsafe {
            env::remove_var("LEAF_ENV");
        }
    }

    #[test]
    #[serial]
    fn test_leaf_env_unknown() {
        unsafe {
            env::set_var("LEAF_ENV", "staging");
        }
        assert_eq!(get_env_file_name(), ".env");
        unsafe {
            env::remove_var("LEAF_ENV");
        }
    }

    #[test]
    #[serial]
    fn test_no_env_set() {
        unsafe {
            env::remove_var("LEAF_ENV_FILE");
            env::remove_var("LEAF_ENV");
        }
        assert_eq!(get_env_file_name(), ".env");
    }
}
