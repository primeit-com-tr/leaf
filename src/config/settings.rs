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
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Helper to clean up the specific environment variables that are prone to leaking
    fn cleanup_hook_env_vars() {
        unsafe {
            env::remove_var("LEAF__HOOKS__PRE_PREPARE_DEPLOYMENT");
            env::remove_var("LEAF__HOOKS__POST_PREPARE_DEPLOYMENT");
            env::remove_var("LEAF__HOOKS__PRE_APPLY_DEPLOYMENT");
            env::remove_var("LEAF__HOOKS__POST_APPLY_DEPLOYMENT");
            env::remove_var("LEAF__HOOKS__PRE_ROLLBACK");
            env::remove_var("LEAF__HOOKS__POST_ROLLBACK");
        }
    }

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

    #[test]
    #[serial]
    fn test_parse_multiline_hooks() {
        cleanup_hook_env_vars(); // Ensure clean slate

        // Create a temporary .env file with multiline hook configuration
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"LEAF__HOOKS__PRE_PREPARE_DEPLOYMENT="
begin system.kill_processes('{{{{ plan }}}}'); end;
begin system.lock_users(); end;
""#
        )
        .unwrap();

        unsafe {
            env::set_var("LEAF_ENV_FILE", temp_file.path());
        }

        let settings = Settings::new().unwrap();

        unsafe {
            env::remove_var("LEAF_ENV_FILE");
        }

        assert!(settings.hooks.pre_prepare_deployment.is_some());
        let hooks = settings.hooks.pre_prepare_deployment.unwrap();
        assert_eq!(hooks.len(), 2);
        assert_eq!(
            hooks[0].trim(),
            "begin system.kill_processes('{{ plan }}'); end;"
        );
        assert_eq!(hooks[1].trim(), "begin system.lock_users(); end;");
    }

    #[test]
    #[serial]
    fn test_parse_single_line_hook() {
        cleanup_hook_env_vars(); // Ensure clean slate

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"LEAF__HOOKS__POST_APPLY_DEPLOYMENT="begin system.notify('Done'); end;""#
        )
        .unwrap();

        unsafe {
            env::set_var("LEAF_ENV_FILE", temp_file.path());
        }

        let settings = Settings::new().unwrap();

        unsafe {
            env::remove_var("LEAF_ENV_FILE");
        }

        assert!(settings.hooks.post_apply_deployment.is_some());
        let hooks = settings.hooks.post_apply_deployment.unwrap();
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0], "begin system.notify('Done'); end;");
    }

    #[test]
    #[serial]
    fn test_parse_multiple_hooks() {
        cleanup_hook_env_vars(); // CRITICAL: Remove polluting variables

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"LEAF__HOOKS__PRE_PREPARE_DEPLOYMENT="
begin system.kill_processes(); end;
begin system.lock_users(); end;
"
LEAF__HOOKS__POST_PREPARE_DEPLOYMENT="
begin system.unlock_users(); end;
begin system.notify('Preparation complete'); end;
""#
        )
        .unwrap();

        unsafe {
            env::set_var("LEAF_ENV_FILE", temp_file.path());
        }

        let settings = Settings::new().unwrap();

        unsafe {
            env::remove_var("LEAF_ENV_FILE");
        }

        // Check pre_prepare_deployment
        assert!(settings.hooks.pre_prepare_deployment.is_some());
        let pre_hooks = settings.hooks.pre_prepare_deployment.unwrap();
        assert_eq!(pre_hooks.len(), 2);
        // This assertion will now pass because the polluting environment variable is gone.
        assert_eq!(pre_hooks[0].trim(), "begin system.kill_processes(); end;");
        assert_eq!(pre_hooks[1].trim(), "begin system.lock_users(); end;");

        // Check post_prepare_deployment
        assert!(settings.hooks.post_prepare_deployment.is_some());
        let post_hooks = settings.hooks.post_prepare_deployment.unwrap();
        assert_eq!(post_hooks.len(), 2);
        assert_eq!(post_hooks[0].trim(), "begin system.unlock_users(); end;");
        assert_eq!(
            post_hooks[1].trim(),
            "begin system.notify('Preparation complete'); end;"
        );
    }

    #[test]
    #[serial]
    fn test_parse_empty_lines_in_hooks() {
        cleanup_hook_env_vars(); // Ensure clean slate

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"LEAF__HOOKS__PRE_ROLLBACK="
begin system.backup(); end;

begin system.notify('Starting rollback'); end;

begin system.lock(); end;
""#
        )
        .unwrap();

        unsafe {
            env::set_var("LEAF_ENV_FILE", temp_file.path());
        }

        let settings = Settings::new().unwrap();

        unsafe {
            env::remove_var("LEAF_ENV_FILE");
        }

        assert!(settings.hooks.pre_rollback.is_some());
        let hooks = settings.hooks.pre_rollback.unwrap();

        // Filter out empty lines
        let non_empty_hooks: Vec<_> = hooks.iter().filter(|h| !h.trim().is_empty()).collect();
        assert_eq!(non_empty_hooks.len(), 3);
    }

    #[test]
    #[serial]
    fn test_hooks_not_set() {
        cleanup_hook_env_vars(); // Ensure hooks are explicitly not set

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "LEAF__DATABASE__URL=sqlite::memory:").unwrap();

        unsafe {
            env::set_var("LEAF_ENV_FILE", temp_file.path());
        }

        let settings = Settings::new().unwrap();

        unsafe {
            env::remove_var("LEAF_ENV_FILE");
        }

        assert!(settings.hooks.pre_prepare_deployment.is_none());
        assert!(settings.hooks.post_prepare_deployment.is_none());
        assert!(settings.hooks.pre_apply_deployment.is_none());
        assert!(settings.hooks.post_apply_deployment.is_none());
        assert!(settings.hooks.pre_rollback.is_none());
        assert!(settings.hooks.post_rollback.is_none());
    }

    #[test]
    #[serial]
    fn test_parse_hooks_with_special_characters() {
        cleanup_hook_env_vars(); // Ensure clean slate

        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"LEAF__HOOKS__POST_ROLLBACK="
begin dbms_output.put_line('Status: {{{{ status }}}}'); end;
begin execute immediate 'ALTER SESSION SET NLS_DATE_FORMAT = ''YYYY-MM-DD'''; end;
""#
        )
        .unwrap();

        unsafe {
            env::set_var("LEAF_ENV_FILE", temp_file.path());
        }

        let settings = Settings::new().unwrap();

        unsafe {
            env::remove_var("LEAF_ENV_FILE");
        }

        assert!(settings.hooks.post_rollback.is_some());
        let hooks = settings.hooks.post_rollback.unwrap();
        assert_eq!(hooks.len(), 2);
        assert!(hooks[0].contains("dbms_output.put_line"));
        assert!(hooks[1].contains("ALTER SESSION"));
    }
}
