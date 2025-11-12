use anyhow::{Context as _, Result};
use rust_embed::RustEmbed;
use tera::{Context, Tera};

#[derive(RustEmbed)]
#[folder = "src/assets/sql/"]
struct Queries;

pub fn get_query(template_name: &str, context: &Context) -> Result<String> {
    let paths_to_try = vec![
        template_name.to_string(),
        format!("src/assets/sql/{}", template_name),
    ];

    for path in paths_to_try {
        if let Some(file) = Queries::get(&path) {
            let template_str = std::str::from_utf8(file.data.as_ref()).unwrap();

            return Tera::default()
                .render_str(template_str, context)
                .context("Failed to render query");
        }
    }

    Err(anyhow::anyhow!("Failed to find query template"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_query() {
        let mut context = Context::new();

        // Provide the required template variables
        context.insert("schemas", "'SCHEMA1', 'SCHEMA2'");
        context.insert("cutoff_date", "2024.10.25:00.00.00");
        context.insert("exclude_object_types", "'SYNONYM', 'LOB'");
        context.insert("exclude_object_names", "'TEMP_TABLE', 'OLD_VIEW'");

        let rendered = get_query("objects.sql.jinja", &context).expect("Failed to get query");

        println!("{}", rendered);
        assert!(rendered.contains("'SCHEMA1', 'SCHEMA2'"));
        assert!(rendered.contains("2024.10.25:00.00.00"));
    }
}
