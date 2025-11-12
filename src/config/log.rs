use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: String,

    #[serde(default)]
    pub dir: Option<String>,

    #[serde(default = "default_console_format")]
    pub console_format: String,

    #[serde(default = "default_true")]
    pub file_enabled: bool,

    #[serde(
        default = "default_ext_level",
        deserialize_with = "deserialize_ext_level"
    )]
    pub ext_level: Option<HashMap<String, String>>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            dir: Some("./logs".to_string()),
            console_format: default_console_format(),
            file_enabled: default_true(),
            ext_level: default_ext_level(),
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_console_format() -> String {
    "pretty".to_string()
}

fn default_true() -> bool {
    true
}

fn default_ext_level() -> Option<HashMap<String, String>> {
    let mut map = HashMap::new();
    map.insert("sqlx".to_string(), "error".to_string());
    Some(map)
}

fn deserialize_ext_level<'de, D>(
    deserializer: D,
) -> Result<Option<HashMap<String, String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;

    let mut map = HashMap::new();
    // Always add sqlx:error as default
    map.insert("sqlx".to_string(), "error".to_string());

    if let Some(s) = s {
        if !s.is_empty() {
            for pair in s.split(',') {
                let pair = pair.trim();
                if let Some((key, value)) = pair.split_once(':') {
                    // This will override the default if sqlx is specified
                    map.insert(key.trim().to_string(), value.trim().to_string());
                }
            }
        }
    }

    Ok(Some(map))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_log_config() {
        let config = LogConfig::default();

        assert_eq!(config.level, "info");
        assert_eq!(config.dir, Some("./logs".to_string()));
        assert_eq!(config.console_format, "pretty");
        assert_eq!(config.file_enabled, true);

        let ext_level = config.ext_level.unwrap();
        assert_eq!(ext_level.len(), 1);
        assert_eq!(ext_level.get("sqlx"), Some(&"error".to_string()));
    }

    #[test]
    fn test_deserialize_empty_ext_level() {
        let json = r#"{
            "level": "debug",
            "ext_level": ""
        }"#;

        let config: LogConfig = serde_json::from_str(json).unwrap();
        let ext_level = config.ext_level.unwrap();

        // Should still have default sqlx:error
        assert_eq!(ext_level.len(), 1);
        assert_eq!(ext_level.get("sqlx"), Some(&"error".to_string()));
    }

    #[test]
    fn test_deserialize_missing_ext_level() {
        let json = r#"{
            "level": "debug"
        }"#;

        let config: LogConfig = serde_json::from_str(json).unwrap();
        let ext_level = config.ext_level.unwrap();

        // Should have default sqlx:error
        assert_eq!(ext_level.len(), 1);
        assert_eq!(ext_level.get("sqlx"), Some(&"error".to_string()));
    }

    #[test]
    fn test_deserialize_single_logger() {
        let json = r#"{
            "level": "debug",
            "ext_level": "another:warn"
        }"#;

        let config: LogConfig = serde_json::from_str(json).unwrap();
        let ext_level = config.ext_level.unwrap();

        // Should have both default sqlx and the new one
        assert_eq!(ext_level.len(), 2);
        assert_eq!(ext_level.get("sqlx"), Some(&"error".to_string()));
        assert_eq!(ext_level.get("another"), Some(&"warn".to_string()));
    }

    #[test]
    fn test_deserialize_multiple_loggers() {
        let json = r#"{
            "level": "debug",
            "ext_level": "hyper:info, tower:debug, another:warn"
        }"#;

        let config: LogConfig = serde_json::from_str(json).unwrap();
        let ext_level = config.ext_level.unwrap();

        // Should have default sqlx plus the three new ones
        assert_eq!(ext_level.len(), 4);
        assert_eq!(ext_level.get("sqlx"), Some(&"error".to_string()));
        assert_eq!(ext_level.get("hyper"), Some(&"info".to_string()));
        assert_eq!(ext_level.get("tower"), Some(&"debug".to_string()));
        assert_eq!(ext_level.get("another"), Some(&"warn".to_string()));
    }

    #[test]
    fn test_deserialize_override_sqlx() {
        let json = r#"{
            "level": "debug",
            "ext_level": "sqlx:info, another:warn"
        }"#;

        let config: LogConfig = serde_json::from_str(json).unwrap();
        let ext_level = config.ext_level.unwrap();

        // sqlx should be overridden to info
        assert_eq!(ext_level.len(), 2);
        assert_eq!(ext_level.get("sqlx"), Some(&"info".to_string()));
        assert_eq!(ext_level.get("another"), Some(&"warn".to_string()));
    }

    #[test]
    fn test_deserialize_with_whitespace() {
        let json = r#"{
            "level": "debug",
            "ext_level": " hyper : info ,  tower : debug "
        }"#;

        let config: LogConfig = serde_json::from_str(json).unwrap();
        let ext_level = config.ext_level.unwrap();

        // Should trim whitespace properly
        assert_eq!(ext_level.len(), 3);
        assert_eq!(ext_level.get("sqlx"), Some(&"error".to_string()));
        assert_eq!(ext_level.get("hyper"), Some(&"info".to_string()));
        assert_eq!(ext_level.get("tower"), Some(&"debug".to_string()));
    }

    #[test]
    fn test_deserialize_invalid_format_ignored() {
        let json = r#"{
            "level": "debug",
            "ext_level": "hyper:info, invalid_no_colon, tower:debug"
        }"#;

        let config: LogConfig = serde_json::from_str(json).unwrap();
        let ext_level = config.ext_level.unwrap();

        // Should skip invalid entries
        assert_eq!(ext_level.len(), 3);
        assert_eq!(ext_level.get("sqlx"), Some(&"error".to_string()));
        assert_eq!(ext_level.get("hyper"), Some(&"info".to_string()));
        assert_eq!(ext_level.get("tower"), Some(&"debug".to_string()));
        assert_eq!(ext_level.get("invalid_no_colon"), None);
    }

    #[test]
    fn test_serialize_log_config() {
        let mut ext_level = HashMap::new();
        ext_level.insert("sqlx".to_string(), "error".to_string());
        ext_level.insert("hyper".to_string(), "info".to_string());

        let config = LogConfig {
            level: "debug".to_string(),
            dir: Some("./logs".to_string()),
            console_format: "json".to_string(),
            file_enabled: true,
            ext_level: Some(ext_level),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"level\":\"debug\""));
        assert!(json.contains("\"sqlx\":\"error\""));
        assert!(json.contains("\"hyper\":\"info\""));
    }

    #[test]
    fn test_deserialize_full_config() {
        let json = r#"{
            "level": "trace",
            "dir": "/var/log",
            "console_format": "json",
            "file_enabled": false,
            "ext_level": "sqlx:warn, hyper:info"
        }"#;

        let config: LogConfig = serde_json::from_str(json).unwrap();

        assert_eq!(config.level, "trace");
        assert_eq!(config.dir, Some("/var/log".to_string()));
        assert_eq!(config.console_format, "json");
        assert_eq!(config.file_enabled, false);

        let ext_level = config.ext_level.unwrap();
        assert_eq!(ext_level.len(), 2);
        assert_eq!(ext_level.get("sqlx"), Some(&"warn".to_string()));
        assert_eq!(ext_level.get("hyper"), Some(&"info".to_string()));
    }
}
