use anyhow::{Context as _, Result};
use rust_embed::RustEmbed;
use tera::{Context, Tera};

#[derive(RustEmbed)]
#[folder = "src/assets/env/"]
struct EnvFiles;

pub fn get_env_file_with_defaults(template_name: &str) -> Result<String> {
    let file = EnvFiles::get(template_name)
        .ok_or_else(|| anyhow::anyhow!("Failed to find env file template: {}", template_name))?;

    let template_str =
        std::str::from_utf8(file.data.as_ref()).context("Failed to parse template as UTF-8")?;

    Tera::default()
        .render_str(template_str, &Context::new())
        .context("Failed to render env file")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_env_file_with_defaults() {
        let result = get_env_file_with_defaults("env.default.jinja");
        if result.is_err() {
            eprintln!("Error: {:?}", result);
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_env_file_with_defaults_not_found() {
        let result = get_env_file_with_defaults("env.default.jinja.not.found");
        assert!(result.is_err());
    }

    #[test]
    fn test_env_file_contents() {
        let result = get_env_file_with_defaults("env.default.jinja");
        assert!(result.is_ok());
        let contents = result.unwrap();
        assert!(contents.contains("DATABASE_URL=sqlite://leaf.db?mode=rwc"));
    }

    // #[test]
    // fn test_list_embedded_files() {
    //     for file in EnvFiles::iter() {
    //         println!("Embedded file: {}", file);
    //     }
    // }
}
