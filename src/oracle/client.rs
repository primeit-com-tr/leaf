use crate::types::Object;
use crate::utils::{format_sql_list, get_query};
use anyhow::{Context as _, Result};
use chrono::NaiveDateTime;
use oracle::Connection;
use tera::Context;
use tracing::debug;

pub struct OracleClient {
    pub conn: Connection,
}

impl OracleClient {
    pub fn connect(username: &str, password: &str, connection_string: &str) -> Result<Self> {
        let conn = Connection::connect(username, password, connection_string).context(format!(
            "Failed to connect to Oracle database with username '{}' and connection string '{}' (password is hidden)",
            username, connection_string
        ))?;
        Ok(Self { conn })
    }

    pub fn get_ddl(&self, object_type: &str, name: &str, schema: &str) -> Result<String> {
        let mut context = Context::new();
        context.insert("object_type", object_type);
        context.insert("name", name);
        context.insert("schema", schema);

        let query =
            get_query("ddl.sql.jinja", &context).context("Failed to render DDL query template")?;
        debug!("DDL query: {}", query);

        let ddl: String = self.conn.query_row(&query, &[])?.get(0)?;
        Ok(ddl)
    }

    pub fn recompile_invalid_objects(&self, parallel_degree: Option<u32>) -> Result<()> {
        let degree = parallel_degree.unwrap_or(0);
        let sql = "BEGIN sys.UTL_RECOMP.recomp_parallel(:1); END;";
        debug!("Executing recompile query with degree {}", degree);
        self.conn.execute(sql, &[&degree])?;
        Ok(())
    }

    pub async fn get_objects(&self) -> Result<Vec<Object>> {
        return self.get_objects_by_status(None).await;
    }

    pub async fn get_objects_by_status(&self, status: Option<&str>) -> Result<Vec<Object>> {
        let mut query = String::from(
            r#"SELECT
                owner, object_name, object_type, created, last_ddl_time, timestamp, status
            FROM dba_objects"#,
        );

        if let Some(status) = status {
            query.push_str(&format!(" WHERE status = '{}'", status));
        }
        query.push_str(" ORDER BY owner, object_name");
        debug!("Query: {}", query);

        let rows = self.conn.query(query.as_str(), &[])?;
        let mut results = Vec::new();

        for row_result in rows {
            let row = row_result?;
            let obj = Object {
                owner: row.get("owner")?,
                object_name: row.get("object_name")?,
                object_type: row.get("object_type")?,
                last_ddl_time: row.get("last_ddl_time")?,
                ddl: None,
            };
            results.push(obj);
        }

        Ok(results)
    }

    pub async fn get_all_users(&self) -> Result<Vec<String>> {
        let query = r#"select username from dba_users order by username"#;
        debug!("Query: {}", query);

        let rows = self.conn.query(query, &[])?;
        let mut schemas = Vec::new();

        for row_result in rows {
            let row = row_result?;
            schemas.push(row.get("username")?);
        }
        Ok(schemas)
    }

    pub async fn get_objects_with_ddls(
        &self,
        schemas: Vec<String>,
        cutoff_date: Option<NaiveDateTime>,
        exclude_object_types: Option<Vec<String>>,
        exclude_object_names: Option<Vec<String>>,
    ) -> Result<Vec<Object>> {
        let mut ctx = Context::new();
        ctx.insert("schemas", &format_sql_list(schemas));
        ctx.insert(
            "exclude_object_types",
            &format_sql_list(exclude_object_types.unwrap_or(vec![])),
        );
        ctx.insert(
            "exclude_object_names",
            &format_sql_list(exclude_object_names.unwrap_or(vec![])),
        );
        if let Some(cutoff_date) = cutoff_date {
            ctx.insert("cutoff_date", &cutoff_date.format("%Y%m%d").to_string());
        }

        let query =
            get_query("objects.sql.jinja", &ctx).context("Failed to render DDL query template")?;
        debug!("Query: {}", query);

        let rows = self.conn.query(query.as_str(), &[])?;
        let mut objects = Vec::new();
        for row_result in rows {
            let row = row_result?;

            let object_type: String = row.get("object_type")?;
            let object_name: String = row.get("object_name")?;
            let owner: String = row.get("schema_name")?;

            let ddl = self.get_ddl(&object_type, &object_name, &owner)?;

            let obj = Object {
                owner,
                object_name,
                object_type,
                last_ddl_time: row.get("last_ddl_time")?,
                ddl: Some(ddl),
            };

            objects.push(obj);
        }

        Ok(objects)
    }

    pub async fn execute(&self, sql: &str) -> Result<()> {
        self.conn.execute(sql, &[])?;
        Ok(())
    }
}
