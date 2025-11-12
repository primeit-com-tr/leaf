use std::collections::{HashMap, HashSet};

use crate::{
    types::{Delta, Object},
    utils::objects_as_map,
};

pub struct Scripts {
    pub scripts: Vec<String>,
    pub rollback_scripts: Vec<String>,
}

fn get_delete_scripts(target: &Object) -> Vec<String> {
    vec![format!(
        "DROP {} {}.{}",
        target.object_type, target.owner, target.object_name
    )]
}

fn get_insert_scripts(source: &Object) -> Vec<String> {
    vec![source.ddl.as_ref().unwrap().clone()]
}

fn extract_columns(ddl: &str) -> Vec<String> {
    let mut columns = Vec::new();

    // Find the column definition section between the first ( and last )
    if let Some(start) = ddl.find('(') {
        if let Some(end) = ddl.rfind(')') {
            let column_section = &ddl[start + 1..end];

            // Split by comma, but be careful about commas inside parentheses (like in CHECK constraints)
            let mut current_col = String::new();
            let mut paren_depth = 0;

            for ch in column_section.chars() {
                match ch {
                    '(' => {
                        paren_depth += 1;
                        current_col.push(ch);
                    }
                    ')' => {
                        paren_depth -= 1;
                        current_col.push(ch);
                    }
                    ',' if paren_depth == 0 => {
                        // This comma is a column separator
                        let col = current_col.trim().to_string();
                        if !col.is_empty() && !is_constraint_line(&col) {
                            columns.push(col);
                        }
                        current_col.clear();
                    }
                    _ => {
                        current_col.push(ch);
                    }
                }
            }

            // Don't forget the last column
            let col = current_col.trim().to_string();
            if !col.is_empty() && !is_constraint_line(&col) {
                columns.push(col);
            }
        }
    }

    columns
}

/// Check if a line is a constraint definition rather than a column
fn is_constraint_line(line: &str) -> bool {
    let upper = line.trim().to_uppercase();
    upper.starts_with("CONSTRAINT")
        || upper.starts_with("PRIMARY KEY")
        || upper.starts_with("FOREIGN KEY")
        || upper.starts_with("UNIQUE")
        || upper.starts_with("CHECK")
        || upper.starts_with("INDEX")
        || line.trim().starts_with("--")
}

/// Parse a column definition into (name, definition)
fn parse_column(col_def: &str) -> Option<(String, String)> {
    let trimmed = col_def.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Split on whitespace to get column name and type
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    // First part is the column name (possibly quoted)
    let name = parts[0].trim_matches('"').to_string();

    // Only return Some if there's actually a data type (at least 2 parts)
    if parts.len() >= 2 {
        Some((name, trimmed.to_string()))
    } else {
        None
    }
}

fn get_update_scripts(source: &Object, target: &Object) -> Vec<String> {
    if source.ddl == target.ddl {
        return Vec::new();
    }

    if source.object_type != "TABLE" {
        return vec![source.ddl.as_ref().unwrap().clone()];
    }

    if source.ddl.is_none() || target.ddl.is_none() {
        return vec![];
    }

    let source_ddl = source.ddl.as_ref().unwrap();
    let target_ddl = target.ddl.as_ref().unwrap();

    let source_cols = extract_columns(&source_ddl);
    let target_cols = extract_columns(&target_ddl);

    // Build maps of column name -> definition
    let mut source_map: HashMap<String, String> = HashMap::new();
    for col in &source_cols {
        if let Some((name, def)) = parse_column(col) {
            source_map.insert(name, def);
        }
    }

    let mut target_map: HashMap<String, String> = HashMap::new();
    for col in &target_cols {
        if let Some((name, def)) = parse_column(col) {
            target_map.insert(name, def);
        }
    }

    let mut scripts = Vec::new();

    // Find columns to ADD (in source but not in target)
    for (name, def) in &source_map {
        if !target_map.contains_key(name) {
            scripts.push(format!(
                "ALTER TABLE {}.{} ADD {}",
                source.owner, source.object_name, def
            ));
        }
    }

    // Find columns to DROP (in target but not in source)
    for (name, _) in &target_map {
        if !source_map.contains_key(name) {
            scripts.push(format!(
                "ALTER TABLE {}.{} DROP COLUMN \"{}\"",
                source.owner, source.object_name, name
            ));
        }
    }

    // Find columns to MODIFY (in both but different)
    for (name, source_def) in &source_map {
        if let Some(target_def) = target_map.get(name) {
            if source_def != target_def {
                scripts.push(format!(
                    "ALTER TABLE {}.{} MODIFY {}",
                    source.owner, source.object_name, source_def
                ));
            }
        }
    }

    scripts
}

pub fn find_scripts(source: Option<Object>, target: Option<Object>) -> Option<Scripts> {
    match (source, target) {
        (None, Some(t)) => Some(Scripts {
            scripts: get_delete_scripts(&t),
            rollback_scripts: get_insert_scripts(&t),
        }),
        (Some(s), None) => Some(Scripts {
            scripts: get_insert_scripts(&s),
            rollback_scripts: get_delete_scripts(&s),
        }),
        (Some(s), Some(t)) => Some(Scripts {
            scripts: get_update_scripts(&s, &t),
            rollback_scripts: get_update_scripts(&t, &s),
        }),
        (None, None) => None,
    }
}

pub fn find_deltas(sources: Vec<Object>, targets: Vec<Object>) -> Vec<Delta> {
    let target_map: HashMap<(String, String, String), Object> = objects_as_map(targets.clone());

    let mut deltas: Vec<Delta> = Vec::new();
    let mut processed_keys: HashSet<(String, String, String)> = HashSet::new();

    // Process sources
    for source in sources {
        let key = (
            source.owner.clone(),
            source.object_name.clone(),
            source.object_type.clone(),
        );
        processed_keys.insert(key.clone());

        let target = target_map.get(&key);
        let scripts = find_scripts(Some(source.clone()), target.cloned());
        deltas.push(Delta {
            object_type: source.object_type.clone(),
            object_name: source.object_name.clone(),
            object_owner: source.owner.clone(),
            source_ddl_time: Some(source.last_ddl_time.clone()),
            source_ddl: source.ddl.clone(),
            target_ddl_time: target.map(|t| t.last_ddl_time.clone()),
            target_ddl: target.map(|t| t.ddl.clone().unwrap_or_default()),
            scripts: scripts
                .as_ref()
                .map(|s| s.scripts.clone())
                .unwrap_or_default(),
            rollback_scripts: scripts.map(|s| s.rollback_scripts).unwrap_or_default(),
            ..Default::default()
        });
    }

    // Process targets that weren't in sources (objects to be deleted)
    for target in targets {
        let key = (
            target.owner.clone(),
            target.object_name.clone(),
            target.object_type.clone(),
        );

        if !processed_keys.contains(&key) {
            let scripts = find_scripts(None, Some(target.clone()));
            deltas.push(Delta {
                object_type: target.object_type.clone(),
                object_name: target.object_name.clone(),
                object_owner: target.owner.clone(),
                source_ddl: None,
                target_ddl: target.ddl.clone(),
                target_ddl_time: Some(target.last_ddl_time.clone()),
                scripts: scripts
                    .as_ref()
                    .map(|s| s.scripts.clone())
                    .unwrap_or_default(),
                rollback_scripts: scripts.map(|s| s.rollback_scripts).unwrap_or_default(),
                ..Default::default()
            });
        }
    }

    deltas
}

pub fn with_disabled_drop_types_excluded(
    deltas: Vec<Delta>,
    disabled_drop_types: Option<Vec<String>>,
) -> Vec<Delta> {
    if disabled_drop_types.is_none() {
        return deltas;
    }

    // Convert the disabled types to a set of keywords for efficient checking.
    // Convert to uppercase once for case-insensitive matching.
    let disabled_types_set: HashSet<String> = disabled_drop_types
        .unwrap()
        .into_iter()
        .map(|s| s.to_uppercase())
        .collect();

    let mut result = Vec::new();

    for delta in deltas {
        let mut new_delta = delta.clone();

        // Use a filter to build the new list of scripts safely.
        let filtered_scripts: Vec<String> = new_delta
            .scripts
            .into_iter()
            .filter(|script| {
                // Convert script to uppercase for case-insensitive search.
                let script_upper = script.to_uppercase();

                // Check if the script should be kept (i.e., does *not* contain a disabled drop operation).
                let is_disabled_drop = disabled_types_set.iter().any(|disabled_type| {
                    // Construct search patterns: "DROP TYPE", "DROP COLUMN", "DROP TABLE", etc.
                    // We check for "DROP " followed by the type name, allowing for text before it (e.g., "ALTER TABLE ... DROP COLUMN").
                    let drop_pattern = format!("DROP {}", disabled_type.to_uppercase());

                    script_upper.contains(&drop_pattern)
                });

                // Keep the script only if it is NOT a disabled drop operation.
                !is_disabled_drop
            })
            .collect();

        new_delta.scripts = filtered_scripts;

        // Skip the entire delta if all scripts were filtered out.
        if !new_delta.scripts.is_empty() {
            result.push(new_delta);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    fn mock_object(owner: &str, name: &str, obj_type: &str, ddl: &str) -> Object {
        Object {
            owner: owner.to_string(),
            object_name: name.to_string(),
            object_type: obj_type.to_string(),
            last_ddl_time: NaiveDateTime::parse_from_str(
                "2025-11-07 12:00:00",
                "%Y-%m-%d %H:%M:%S",
            )
            .unwrap(),
            ddl: Some(ddl.to_string()),
        }
    }

    fn mock_datetime() -> NaiveDateTime {
        NaiveDateTime::parse_from_str("2024-01-01 12:00:00", "%Y-%m-%d %H:%M:%S").unwrap()
    }

    fn mock_objects() -> Vec<Object> {
        vec![
            Object {
                owner: "LEAF".to_string(),
                object_name: "EMP_PKG".to_string(),
                object_type: "PACKAGE".to_string(),
                last_ddl_time: mock_datetime(),
                ddl: Some(
                    r#"CREATE OR REPLACE PACKAGE LEAF.EMP_PKG AS
  PROCEDURE HIRE_EMP(p_name VARCHAR2, p_salary NUMBER);
  FUNCTION GET_TOTAL_EMPLOYEES RETURN NUMBER;
END EMP_PKG;"#
                        .to_string(),
                ),
            },
            Object {
                owner: "LEAF".to_string(),
                object_name: "UPDATE_SALARY".to_string(),
                object_type: "PROCEDURE".to_string(),
                last_ddl_time: mock_datetime(),
                ddl: Some(
                    r#"CREATE OR REPLACE PROCEDURE LEAF.UPDATE_SALARY (
  p_emp_id IN NUMBER,
  p_new_salary IN NUMBER
) AS
BEGIN
  UPDATE LEAF.EMP
  SET SALARY = p_new_salary
  WHERE EMP_ID = p_emp_id;
END UPDATE_SALARY;"#
                        .to_string(),
                ),
            },
            Object {
                owner: "LEAF".to_string(),
                object_name: "GET_EMP_DEPT".to_string(),
                object_type: "FUNCTION".to_string(),
                last_ddl_time: mock_datetime(),
                ddl: Some(
                    r#"CREATE OR REPLACE FUNCTION LEAF.GET_EMP_DEPT (
  p_emp_id IN NUMBER
) RETURN VARCHAR2 AS
  v_dept_name VARCHAR2(100);
BEGIN
  SELECT D.DEPARTMENT_NAME INTO v_dept_name
  FROM LEAF.EMP E
  JOIN LEAF.DEPT D ON E.DEPT_ID = D.DEPT_ID
  WHERE E.EMP_ID = p_emp_id;
  RETURN v_dept_name;
END GET_EMP_DEPT;"#
                        .to_string(),
                ),
            },
            Object {
                owner: "LEAF".to_string(),
                object_name: "BI_EMP".to_string(),
                object_type: "TRIGGER".to_string(),
                last_ddl_time: mock_datetime(),
                ddl: Some(
                    r#"CREATE OR REPLACE TRIGGER LEAF.BI_EMP
  BEFORE INSERT ON LEAF.EMP
  FOR EACH ROW
BEGIN
  :NEW.EMP_ID := LEAF.EMP_SEQ.NEXTVAL;
END;"#
                        .to_string(),
                ),
            },
            Object {
                owner: "LEAF".to_string(),
                object_name: "EMP_SEQ".to_string(),
                object_type: "SEQUENCE".to_string(),
                last_ddl_time: mock_datetime(),
                ddl: Some(
                    r#"CREATE SEQUENCE LEAF.EMP_SEQ
  START WITH 1
  INCREMENT BY 1
  NOCACHE
  NOCYCLE;"#
                        .to_string(),
                ),
            },
            Object {
                owner: "LEAF".to_string(),
                object_name: "IDX_EMP_NAME".to_string(),
                object_type: "INDEX".to_string(),
                last_ddl_time: mock_datetime(),
                ddl: Some(
                    r#"CREATE INDEX LEAF.IDX_EMP_NAME
  ON LEAF.EMP (EMP_NAME ASC)
  TABLESPACE USERS;"#
                        .to_string(),
                ),
            },
        ]
    }

    #[test]
    fn test_find_deltas_non_table_view() {
        let sources = mock_objects();

        // simulate target objects being different (empty DDLs or old DDLs)
        let mut targets = sources.clone();
        for t in &mut targets {
            t.ddl = Some("-- old DDL".to_string());
        }

        let deltas = find_deltas(sources.clone(), targets);

        // Every delta should have scripts equal to source DDL since type != TABLE/VIEW
        for (i, delta) in deltas.iter().enumerate() {
            let source_ddl = sources[i].ddl.as_ref().unwrap();
            assert_eq!(delta.scripts, vec![source_ddl.clone()]);
        }
    }

    #[test]
    fn test_find_deltas_object_deletion() {
        let sources = vec![]; // nothing exists in source
        let targets = mock_objects(); // all objects exist in target

        let deltas = find_deltas(sources, targets.clone());

        // Every delta should have scripts = DROP <TYPE> <OWNER>.<NAME>
        for (i, delta) in deltas.iter().enumerate() {
            let target = &targets[i];
            let expected = format!(
                "DROP {} {}.{}",
                target.object_type, target.owner, target.object_name
            );
            assert!(delta.scripts.contains(&expected));
        }
    }

    // ==================== Basic Script Generation Tests ====================

    #[test]
    fn test_get_delete_scripts() {
        let obj = mock_object("HR", "EMP", "TABLE", "CREATE TABLE EMP (ID INT)");
        let scripts = get_delete_scripts(&obj);
        assert_eq!(scripts, vec!["DROP TABLE HR.EMP"]);
    }

    #[test]
    fn test_get_insert_scripts() {
        let obj = mock_object("HR", "EMP", "TABLE", "CREATE TABLE EMP (ID INT)");
        let scripts = get_insert_scripts(&obj);
        assert_eq!(scripts, vec!["CREATE TABLE EMP (ID INT)"]);
    }

    // ==================== Column Extraction Tests ====================

    #[test]
    fn test_extract_columns_quoted_identifiers() {
        let ddl = r#"
        CREATE TABLE "EMP" (
            "ID" NUMBER,
            "NAME" VARCHAR2(100),
            "AGE" NUMBER
        );
        "#;

        let cols = extract_columns(ddl);
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0], r#""ID" NUMBER"#);
        assert_eq!(cols[1], r#""NAME" VARCHAR2(100)"#);
        assert_eq!(cols[2], r#""AGE" NUMBER"#);
    }

    #[test]
    fn test_extract_columns_unquoted_identifiers() {
        let ddl = r#"
        CREATE TABLE EMP (
            ID INT,
            NAME VARCHAR2(200),
            AGE NUMBER
        )
        "#;

        let cols = extract_columns(ddl);
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0], "ID INT");
        assert_eq!(cols[1], "NAME VARCHAR2(200)");
        assert_eq!(cols[2], "AGE NUMBER");
    }

    #[test]
    fn test_extract_columns_single_line_definition() {
        let ddl = r#"CREATE TABLE "LEAF"."EMP" ("ID" CHAR(2))"#;

        let cols = extract_columns(ddl);
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0], r#""ID" CHAR(2)"#);
    }

    #[test]
    fn test_extract_columns_mixed_line_format() {
        let ddl = r#"
        CREATE TABLE "LEAF"."EMP"
           (	"ID" CHAR(2),
           "NAME" VARCHAR2(100)
           )
        "#;

        let cols = extract_columns(ddl);
        assert_eq!(cols.len(), 2);
        assert_eq!(cols[0], r#""ID" CHAR(2)"#);
        assert_eq!(cols[1], r#""NAME" VARCHAR2(100)"#);
    }

    #[test]
    fn test_extract_columns_with_constraints() {
        let ddl = r#"
        CREATE TABLE "EMP" (
            "ID" NUMBER,
            "NAME" VARCHAR2(100),
            CONSTRAINT pk_emp PRIMARY KEY (ID)
        )
        "#;

        let cols = extract_columns(ddl);
        assert_eq!(cols.len(), 2);
        assert_eq!(cols[0], r#""ID" NUMBER"#);
        assert_eq!(cols[1], r#""NAME" VARCHAR2(100)"#);
    }

    #[test]
    fn test_extract_columns_empty_table() {
        let ddl = "CREATE TABLE EMP ()";
        let cols = extract_columns(ddl);
        assert_eq!(cols.len(), 0);
    }

    // ==================== Column Parsing Tests ====================

    #[test]
    fn test_parse_column_quoted() {
        let input = r#""ID" NUMBER NOT NULL"#;
        let parsed = parse_column(input).unwrap();
        assert_eq!(parsed.0, "ID");
        assert_eq!(parsed.1, r#""ID" NUMBER NOT NULL"#);
    }

    #[test]
    fn test_parse_column_unquoted() {
        let input = "NAME VARCHAR2(100)";
        let parsed = parse_column(input).unwrap();
        assert_eq!(parsed.0, "NAME");
        assert_eq!(parsed.1, "NAME VARCHAR2(100)");
    }

    #[test]
    fn test_parse_column_invalid() {
        assert!(parse_column("invalid_column_def").is_none());
        assert!(parse_column("").is_none());
    }

    // ==================== Update Scripts Tests ====================

    #[test]
    fn test_get_update_scripts_identical_tables() {
        let ddl = r#"
        CREATE TABLE "EMP" (
            "ID" NUMBER,
            "NAME" VARCHAR2(100)
        )
        "#;
        let s = mock_object("HR", "EMP", "TABLE", ddl);
        let t = s.clone();

        let scripts = get_update_scripts(&s, &t);
        assert!(scripts.is_empty());
    }

    #[test]
    fn test_get_update_scripts_add_column_quoted() {
        let ddl_source = r#"
        CREATE TABLE "EMP" (
            "ID" NUMBER,
            "NAME" VARCHAR2(100),
            "AGE" NUMBER
        )
        "#;

        let ddl_target = r#"
        CREATE TABLE "EMP" (
            "ID" NUMBER,
            "NAME" VARCHAR2(100)
        )
        "#;

        let s = mock_object("HR", "EMP", "TABLE", ddl_source);
        let t = mock_object("HR", "EMP", "TABLE", ddl_target);

        let scripts = get_update_scripts(&s, &t);
        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0], r#"ALTER TABLE HR.EMP ADD "AGE" NUMBER"#);
    }

    #[test]
    fn test_get_update_scripts_add_column_unquoted() {
        let ddl_source = r#"
        CREATE TABLE EMP (
            ID INT,
            NAME VARCHAR2(100),
            AGE NUMBER
        )
        "#;

        let ddl_target = r#"
        CREATE TABLE EMP (
            ID INT,
            NAME VARCHAR2(100)
        )
        "#;

        let s = mock_object("HR", "EMP", "TABLE", ddl_source);
        let t = mock_object("HR", "EMP", "TABLE", ddl_target);

        let scripts = get_update_scripts(&s, &t);
        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0], "ALTER TABLE HR.EMP ADD AGE NUMBER");
    }

    #[test]
    fn test_get_update_scripts_drop_column_quoted() {
        let ddl_source = r#"
        CREATE TABLE "EMP" (
            "ID" NUMBER
        )
        "#;

        let ddl_target = r#"
        CREATE TABLE "EMP" (
            "ID" NUMBER,
            "NAME" VARCHAR2(100)
        )
        "#;

        let s = mock_object("HR", "EMP", "TABLE", ddl_source);
        let t = mock_object("HR", "EMP", "TABLE", ddl_target);

        let scripts = get_update_scripts(&s, &t);
        assert_eq!(scripts.len(), 1);
        assert!(scripts.iter().any(|s| s.contains("DROP COLUMN \"NAME\"")));
    }

    #[test]
    fn test_get_update_scripts_drop_column_unquoted() {
        let ddl_source = r#"
        CREATE TABLE EMP (
            ID INT
        )
        "#;

        let ddl_target = r#"
        CREATE TABLE EMP (
            ID INT,
            NAME VARCHAR2(100)
        )
        "#;

        let s = mock_object("HR", "EMP", "TABLE", ddl_source);
        let t = mock_object("HR", "EMP", "TABLE", ddl_target);

        let scripts = get_update_scripts(&s, &t);
        assert_eq!(scripts.len(), 1);
        assert!(scripts.iter().any(|s| s.contains("DROP COLUMN \"NAME\"")));
    }

    #[test]
    fn test_get_update_scripts_modify_column_quoted() {
        let ddl_source = r#"
        CREATE TABLE "EMP" (
            "ID" NUMBER,
            "NAME" VARCHAR2(200)
        )
        "#;

        let ddl_target = r#"
        CREATE TABLE "EMP" (
            "ID" NUMBER,
            "NAME" VARCHAR2(100)
        )
        "#;

        let s = mock_object("HR", "EMP", "TABLE", ddl_source);
        let t = mock_object("HR", "EMP", "TABLE", ddl_target);

        let scripts = get_update_scripts(&s, &t);
        assert_eq!(scripts.len(), 1);
        assert!(scripts.iter().any(|s| s.contains("MODIFY")));
        assert!(scripts[0].contains(r#""NAME" VARCHAR2(200)"#));
    }

    #[test]
    fn test_get_update_scripts_modify_column_unquoted() {
        let ddl_source = r#"
        CREATE TABLE EMP (
            ID INT,
            NAME VARCHAR2(200)
        )
        "#;

        let ddl_target = r#"
        CREATE TABLE EMP (
            ID INT,
            NAME VARCHAR2(100)
        )
        "#;

        let s = mock_object("HR", "EMP", "TABLE", ddl_source);
        let t = mock_object("HR", "EMP", "TABLE", ddl_target);

        let scripts = get_update_scripts(&s, &t);
        assert_eq!(scripts.len(), 1);
        assert!(scripts.iter().any(|s| s.contains("MODIFY")));
        assert!(scripts[0].contains("NAME VARCHAR2(200)"));
    }

    #[test]
    fn test_get_update_scripts_modify_data_type() {
        let ddl_source = r#"
        CREATE TABLE "LEAF"."EMP"
           ("ID" CHAR(2))
        "#;

        let ddl_target = r#"
        CREATE TABLE "LEAF"."EMP"
           ("ID" NUMBER)
        "#;

        let s = mock_object("LEAF", "EMP", "TABLE", ddl_source);
        let t = mock_object("LEAF", "EMP", "TABLE", ddl_target);

        let scripts = get_update_scripts(&s, &t);
        assert_eq!(scripts.len(), 1);
        assert!(scripts[0].contains("MODIFY"));
        assert!(scripts[0].contains(r#""ID" CHAR(2)"#));
    }

    #[test]
    fn test_get_update_scripts_multiple_changes() {
        let ddl_source = r#"
        CREATE TABLE "EMP" (
            "ID" NUMBER,
            "NAME" VARCHAR2(200),
            "DEPT" VARCHAR2(50)
        )
        "#;

        let ddl_target = r#"
        CREATE TABLE "EMP" (
            "ID" NUMBER,
            "NAME" VARCHAR2(100),
            "AGE" NUMBER
        )
        "#;

        let s = mock_object("HR", "EMP", "TABLE", ddl_source);
        let t = mock_object("HR", "EMP", "TABLE", ddl_target);

        let scripts = get_update_scripts(&s, &t);

        // Should have: ADD DEPT, DROP AGE, MODIFY NAME
        assert_eq!(scripts.len(), 3);
        assert!(
            scripts
                .iter()
                .any(|s| s.contains("ADD") && s.contains("DEPT"))
        );
        assert!(scripts.iter().any(|s| s.contains("DROP COLUMN \"AGE\"")));
        assert!(
            scripts
                .iter()
                .any(|s| s.contains("MODIFY") && s.contains("NAME"))
        );
    }

    #[test]
    fn test_get_update_scripts_non_table_object() {
        let ddl_source = "CREATE VIEW EMP_VIEW AS SELECT * FROM EMP";
        let ddl_target = "CREATE VIEW EMP_VIEW AS SELECT ID FROM EMP";

        let s = mock_object("HR", "EMP_VIEW", "VIEW", ddl_source);
        let t = mock_object("HR", "EMP_VIEW", "VIEW", ddl_target);

        let scripts = get_update_scripts(&s, &t);
        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0], ddl_source);
    }

    // ==================== Find Scripts Tests ====================

    #[test]
    fn test_find_scripts_new_table() {
        let s = mock_object("HR", "EMP", "TABLE", "CREATE TABLE EMP (ID INT)");

        let result = find_scripts(Some(s.clone()), None).unwrap();
        assert!(result.scripts[0].contains("CREATE TABLE"));
        assert!(result.rollback_scripts[0].contains("DROP TABLE"));
    }

    #[test]
    fn test_find_scripts_deleted_table() {
        let t = mock_object("HR", "EMP", "TABLE", "CREATE TABLE EMP (ID INT)");

        let result = find_scripts(None, Some(t.clone())).unwrap();
        assert!(result.scripts[0].contains("DROP TABLE"));
        assert!(result.rollback_scripts[0].contains("CREATE TABLE"));
    }

    #[test]
    fn test_find_scripts_modified_table() {
        let s = mock_object(
            "HR",
            "EMP",
            "TABLE",
            r#"CREATE TABLE "EMP" ("ID" NUMBER, "NAME" VARCHAR2(200))"#,
        );
        let t = mock_object(
            "HR",
            "EMP",
            "TABLE",
            r#"CREATE TABLE "EMP" ("ID" NUMBER, "NAME" VARCHAR2(100))"#,
        );

        let result = find_scripts(Some(s), Some(t)).unwrap();
        assert!(result.scripts.iter().any(|s| s.contains("MODIFY")));
        assert!(result.rollback_scripts.iter().any(|s| s.contains("MODIFY")));
    }

    #[test]
    fn test_find_scripts_none_none() {
        let result = find_scripts(None, None);
        assert!(result.is_none());
    }

    // ==================== Find Deltas Tests ====================

    #[test]
    fn test_find_deltas_add_table() {
        let s = mock_object("HR", "EMP", "TABLE", "CREATE TABLE EMP (ID INT)");

        let deltas = find_deltas(vec![s.clone()], vec![]);

        assert_eq!(deltas.len(), 1);
        let delta = &deltas[0];
        assert_eq!(delta.object_name, "EMP");
        assert_eq!(delta.object_owner, "HR");
        assert!(delta.scripts.iter().any(|s| s.contains("CREATE TABLE")));
        assert!(
            delta
                .rollback_scripts
                .iter()
                .any(|s| s.contains("DROP TABLE"))
        );
        assert!(delta.source_ddl.is_some());
        assert!(delta.target_ddl.is_none() || delta.target_ddl.as_ref().unwrap().is_empty());
    }

    #[test]
    fn test_find_deltas_drop_table() {
        let t = mock_object("HR", "DEPT", "TABLE", "CREATE TABLE DEPT (ID INT)");

        let deltas = find_deltas(vec![], vec![t.clone()]);

        assert_eq!(deltas.len(), 1);
        let delta = &deltas[0];
        assert_eq!(delta.object_name, "DEPT");
        assert_eq!(delta.object_owner, "HR");
        assert!(delta.scripts.iter().any(|s| s.contains("DROP TABLE")));
        assert!(
            delta
                .rollback_scripts
                .iter()
                .any(|s| s.contains("CREATE TABLE"))
        );
        assert!(delta.source_ddl.is_none());
        assert!(delta.target_ddl.is_some());
    }

    #[test]
    fn test_find_deltas_add_and_drop_tables() {
        let s = mock_object("HR", "EMP", "TABLE", "CREATE TABLE EMP (ID INT)");
        let t = mock_object("HR", "DEPT", "TABLE", "CREATE TABLE DEPT (ID INT)");

        let deltas = find_deltas(vec![s.clone()], vec![t.clone()]);

        assert_eq!(deltas.len(), 2);

        let emp = deltas.iter().find(|d| d.object_name == "EMP").unwrap();
        assert!(emp.scripts.iter().any(|s| s.contains("CREATE TABLE")));
        assert!(emp.source_ddl.is_some());
        assert!(emp.target_ddl.is_none() || emp.target_ddl.as_ref().unwrap().is_empty());

        let dept = deltas.iter().find(|d| d.object_name == "DEPT").unwrap();
        assert!(dept.scripts.iter().any(|s| s.contains("DROP TABLE")));
        assert!(dept.source_ddl.is_none());
        assert!(dept.target_ddl.is_some());
    }

    #[test]
    fn test_find_deltas_modified_table() {
        let s = mock_object(
            "HR",
            "EMP",
            "TABLE",
            r#"CREATE TABLE EMP (
                ID INT,
                NAME VARCHAR2(200)
            )"#,
        );
        let t = mock_object(
            "HR",
            "EMP",
            "TABLE",
            r#"CREATE TABLE EMP (
                ID INT,
                NAME VARCHAR2(100)
            )"#,
        );

        let deltas = find_deltas(vec![s.clone()], vec![t.clone()]);

        assert_eq!(deltas.len(), 1);
        let delta = &deltas[0];
        assert_eq!(delta.object_name, "EMP");
        assert!(delta.scripts.iter().any(|s| s.contains("MODIFY")));
        assert!(delta.rollback_scripts.iter().any(|s| s.contains("MODIFY")));
        assert!(delta.source_ddl.is_some());
        assert!(delta.target_ddl.is_some());
    }

    #[test]
    fn test_find_deltas_no_changes() {
        let ddl = "CREATE TABLE EMP (ID INT)";
        let s = mock_object("HR", "EMP", "TABLE", ddl);
        let t = s.clone();

        let deltas = find_deltas(vec![s], vec![t]);

        assert_eq!(deltas.len(), 1);
        let delta = &deltas[0];
        assert!(delta.scripts.is_empty());
        assert!(delta.rollback_scripts.is_empty());
    }

    #[test]
    fn test_debug_multiple_objects() {
        let s1 = mock_object(
            "HR",
            "EMP",
            "TABLE",
            "CREATE TABLE EMP (ID INT, AGE NUMBER)",
        );
        let t1 = mock_object("HR", "EMP", "TABLE", "CREATE TABLE EMP (ID INT)");

        let scripts = get_update_scripts(&s1, &t1);

        assert!(scripts.iter().any(|s| s.contains("ADD")));
        assert!(scripts.iter().any(|s| s.contains("AGE")));
    }

    #[test]
    fn test_find_deltas_multiple_objects() {
        // Source = desired state, Target = current state
        let s1 = mock_object(
            "HR",
            "EMP",
            "TABLE",
            "CREATE TABLE EMP (ID INT, AGE NUMBER)",
        );
        let s2 = mock_object(
            "HR",
            "DEPT",
            "TABLE",
            "CREATE TABLE DEPT (ID INT, NAME VARCHAR2(100))",
        );

        let t1 = mock_object("HR", "EMP", "TABLE", "CREATE TABLE EMP (ID INT)");
        let t2 = mock_object("HR", "LOCATION", "TABLE", "CREATE TABLE LOCATION (ID INT)");

        let deltas = find_deltas(vec![s1, s2], vec![t1, t2]);

        // EMP: modified (add AGE column - it's in source but not in target)
        // DEPT: new (exists in source, not in target) - should CREATE
        // LOCATION: deleted (exists in target, not in source) - should DROP
        assert_eq!(deltas.len(), 3);

        let emp = deltas.iter().find(|d| d.object_name == "EMP").unwrap();
        assert!(
            emp.scripts
                .iter()
                .any(|s| s.contains("ADD") && s.contains("AGE"))
        );

        let dept = deltas.iter().find(|d| d.object_name == "DEPT").unwrap();
        assert!(dept.scripts.iter().any(|s| s.contains("CREATE TABLE")));

        let location = deltas.iter().find(|d| d.object_name == "LOCATION").unwrap();
        assert!(location.scripts.iter().any(|s| s.contains("DROP TABLE")));
    }

    #[test]
    fn test_find_deltas_different_owners() {
        let s1 = mock_object("HR", "EMP", "TABLE", "CREATE TABLE EMP (ID INT)");
        let s2 = mock_object(
            "SALES",
            "EMP",
            "TABLE",
            "CREATE TABLE EMP (ID INT, NAME VARCHAR2(100))",
        );

        let deltas = find_deltas(vec![s1, s2], vec![]);

        assert_eq!(deltas.len(), 2);
        assert!(
            deltas
                .iter()
                .any(|d| d.object_owner == "HR" && d.object_name == "EMP")
        );
        assert!(
            deltas
                .iter()
                .any(|d| d.object_owner == "SALES" && d.object_name == "EMP")
        );
    }

    #[test]
    fn test_find_deltas_empty_inputs() {
        let deltas = find_deltas(vec![], vec![]);
        assert_eq!(deltas.len(), 0);
    }

    #[test]
    fn test_find_deltas_with_views() {
        let s = mock_object(
            "HR",
            "EMP_VIEW",
            "VIEW",
            "CREATE VIEW EMP_VIEW AS SELECT * FROM EMP",
        );
        let t = mock_object(
            "HR",
            "EMP_VIEW",
            "VIEW",
            "CREATE VIEW EMP_VIEW AS SELECT ID FROM EMP",
        );

        let deltas = find_deltas(vec![s], vec![t]);

        assert_eq!(deltas.len(), 1);
        let delta = &deltas[0];
        assert_eq!(delta.object_type, "VIEW");
        // Views should be recreated, not altered
        assert!(delta.scripts.iter().any(|s| s.contains("CREATE VIEW")));
    }

    // ==================== Edge Cases and Integration Tests ====================

    #[test]
    fn test_extract_columns_with_trailing_commas() {
        let ddl = r#"
        CREATE TABLE "EMP" (
            "ID" NUMBER,
            "NAME" VARCHAR2(100),
        )
        "#;

        let cols = extract_columns(ddl);
        assert_eq!(cols.len(), 2);
        assert_eq!(cols[0], r#""ID" NUMBER"#);
        assert_eq!(cols[1], r#""NAME" VARCHAR2(100)"#);
    }

    #[test]
    fn test_extract_columns_complex_data_types() {
        let ddl = r#"
        CREATE TABLE "EMPLOYEE" (
            "ID" NUMBER(10,0) NOT NULL,
            "HIRE_DATE" DATE DEFAULT SYSDATE,
            "SALARY" NUMBER(10,2),
            "DESCRIPTION" CLOB,
            "IS_ACTIVE" CHAR(1) CHECK (IS_ACTIVE IN ('Y','N'))
        )
        "#;

        let cols = extract_columns(ddl);
        assert_eq!(cols.len(), 5);
        assert!(cols[0].contains("NUMBER(10,0) NOT NULL"));
        assert!(cols[1].contains("DATE DEFAULT SYSDATE"));
        assert!(cols[2].contains("NUMBER(10,2)"));
        assert!(cols[3].contains("CLOB"));
        assert!(cols[4].contains("CHECK"));
    }

    #[test]
    fn test_get_update_scripts_add_multiple_columns() {
        let ddl_source = r#"
        CREATE TABLE "EMP" (
            "ID" NUMBER,
            "NAME" VARCHAR2(100),
            "EMAIL" VARCHAR2(200),
            "PHONE" VARCHAR2(20)
        )
        "#;

        let ddl_target = r#"
        CREATE TABLE "EMP" (
            "ID" NUMBER,
            "NAME" VARCHAR2(100)
        )
        "#;

        let s = mock_object("HR", "EMP", "TABLE", ddl_source);
        let t = mock_object("HR", "EMP", "TABLE", ddl_target);

        let scripts = get_update_scripts(&s, &t);
        assert_eq!(scripts.len(), 2);
        assert!(
            scripts
                .iter()
                .any(|s| s.contains("ADD") && s.contains("EMAIL"))
        );
        assert!(
            scripts
                .iter()
                .any(|s| s.contains("ADD") && s.contains("PHONE"))
        );
    }

    #[test]
    fn test_get_update_scripts_drop_multiple_columns() {
        let ddl_source = r#"
        CREATE TABLE "EMP" (
            "ID" NUMBER
        )
        "#;

        let ddl_target = r#"
        CREATE TABLE "EMP" (
            "ID" NUMBER,
            "NAME" VARCHAR2(100),
            "EMAIL" VARCHAR2(200)
        )
        "#;

        let s = mock_object("HR", "EMP", "TABLE", ddl_source);
        let t = mock_object("HR", "EMP", "TABLE", ddl_target);

        let scripts = get_update_scripts(&s, &t);
        assert_eq!(scripts.len(), 2);
        assert!(scripts.iter().any(|s| s.contains("DROP COLUMN \"NAME\"")));
        assert!(scripts.iter().any(|s| s.contains("DROP COLUMN \"EMAIL\"")));
    }

    #[test]
    fn test_get_update_scripts_case_sensitivity() {
        let ddl_source = r#"
        CREATE TABLE "EMP" (
            "Id" NUMBER,
            "Name" VARCHAR2(100)
        )
        "#;

        let ddl_target = r#"
        CREATE TABLE "EMP" (
            "ID" NUMBER,
            "NAME" VARCHAR2(100)
        )
        "#;

        let s = mock_object("HR", "EMP", "TABLE", ddl_source);
        let t = mock_object("HR", "EMP", "TABLE", ddl_target);

        let scripts = get_update_scripts(&s, &t);
        // Different case = different columns in Oracle
        assert_eq!(scripts.len(), 4); // Add Id, Name, Drop ID, NAME
    }

    #[test]
    fn test_rollback_scripts_symmetry() {
        let ddl_source = r#"
        CREATE TABLE "EMP" (
            "ID" NUMBER,
            "NAME" VARCHAR2(200)
        )
        "#;

        let ddl_target = r#"
        CREATE TABLE "EMP" (
            "ID" NUMBER,
            "NAME" VARCHAR2(100)
        )
        "#;

        let s = mock_object("HR", "EMP", "TABLE", ddl_source);
        let t = mock_object("HR", "EMP", "TABLE", ddl_target);

        let forward = get_update_scripts(&s, &t);
        let backward = get_update_scripts(&t, &s);

        // Forward should modify NAME to 200
        assert!(forward.iter().any(|s| s.contains("VARCHAR2(200)")));

        // Backward should modify NAME to 100
        assert!(backward.iter().any(|s| s.contains("VARCHAR2(100)")));
    }

    #[test]
    fn test_find_deltas_preserves_ddl_times() {
        let mut s = mock_object("HR", "EMP", "TABLE", "CREATE TABLE EMP (ID INT)");
        s.last_ddl_time =
            NaiveDateTime::parse_from_str("2025-11-01 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

        let mut t = mock_object(
            "HR",
            "EMP",
            "TABLE",
            "CREATE TABLE EMP (ID INT, NAME VARCHAR2(100))",
        );
        t.last_ddl_time =
            NaiveDateTime::parse_from_str("2025-11-05 15:30:00", "%Y-%m-%d %H:%M:%S").unwrap();

        let deltas = find_deltas(vec![s.clone()], vec![t.clone()]);

        let delta = &deltas[0];
        assert_eq!(delta.source_ddl_time, Some(s.last_ddl_time));
        assert_eq!(delta.target_ddl_time, Some(t.last_ddl_time));
    }

    #[test]
    fn test_find_deltas_complex_scenario() {
        // Simulate a real-world migration scenario
        let sources = vec![
            // Modified table
            mock_object(
                "HR",
                "EMPLOYEES",
                "TABLE",
                r#"CREATE TABLE "EMPLOYEES" ("ID" NUMBER, "NAME" VARCHAR2(200), "DEPT_ID" NUMBER)"#,
            ),
            // New table
            mock_object(
                "HR",
                "DEPARTMENTS",
                "TABLE",
                r#"CREATE TABLE "DEPARTMENTS" ("ID" NUMBER, "NAME" VARCHAR2(100))"#,
            ),
            // Unchanged table
            mock_object(
                "HR",
                "LOCATIONS",
                "TABLE",
                r#"CREATE TABLE "LOCATIONS" ("ID" NUMBER)"#,
            ),
            // Modified view
            mock_object(
                "HR",
                "EMP_VIEW",
                "VIEW",
                "CREATE VIEW EMP_VIEW AS SELECT ID, NAME FROM EMPLOYEES",
            ),
        ];

        let targets = vec![
            // Was modified
            mock_object(
                "HR",
                "EMPLOYEES",
                "TABLE",
                r#"CREATE TABLE "EMPLOYEES" ("ID" NUMBER, "NAME" VARCHAR2(100))"#,
            ),
            // Will be dropped
            mock_object(
                "HR",
                "SALARIES",
                "TABLE",
                r#"CREATE TABLE "SALARIES" ("EMP_ID" NUMBER, "AMOUNT" NUMBER)"#,
            ),
            // Unchanged
            mock_object(
                "HR",
                "LOCATIONS",
                "TABLE",
                r#"CREATE TABLE "LOCATIONS" ("ID" NUMBER)"#,
            ),
            // Was modified
            mock_object(
                "HR",
                "EMP_VIEW",
                "VIEW",
                "CREATE VIEW EMP_VIEW AS SELECT ID FROM EMPLOYEES",
            ),
        ];

        let deltas = find_deltas(sources, targets);

        assert_eq!(deltas.len(), 5);

        // Check EMPLOYEES was modified
        let emp = deltas
            .iter()
            .find(|d| d.object_name == "EMPLOYEES")
            .unwrap();
        assert!(!emp.scripts.is_empty());
        assert!(
            emp.scripts
                .iter()
                .any(|s| s.contains("MODIFY") || s.contains("ADD"))
        );

        // Check DEPARTMENTS was added
        let dept = deltas
            .iter()
            .find(|d| d.object_name == "DEPARTMENTS")
            .unwrap();
        assert!(dept.scripts.iter().any(|s| s.contains("CREATE TABLE")));
        assert!(dept.source_ddl.is_some());
        assert!(dept.target_ddl.is_none() || dept.target_ddl.as_ref().unwrap().is_empty());

        // Check SALARIES was dropped
        let sal = deltas.iter().find(|d| d.object_name == "SALARIES").unwrap();
        assert!(sal.scripts.iter().any(|s| s.contains("DROP TABLE")));
        assert!(sal.source_ddl.is_none());
        assert!(sal.target_ddl.is_some());

        // Check LOCATIONS unchanged
        let loc = deltas
            .iter()
            .find(|d| d.object_name == "LOCATIONS")
            .unwrap();
        assert!(loc.scripts.is_empty());

        // Check EMP_VIEW was modified
        let view = deltas.iter().find(|d| d.object_name == "EMP_VIEW").unwrap();
        assert!(view.scripts.iter().any(|s| s.contains("CREATE VIEW")));
    }
}

#[cfg(test)]
mod test_excluded_drop_types {
    use super::*;

    // Helper function to create a basic Delta
    // FIX: Changed signature from Vec<&str> to &[&str] and used .iter() instead of .into_iter()
    fn create_delta(scripts: &[&str]) -> Delta {
        Delta {
            scripts: scripts.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    // Helper function to create Option<Vec<String>>
    fn disabled_list(items: &[&str]) -> Option<Vec<String>> {
        Some(items.iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn test_no_disabled_types_returns_original_deltas() {
        let deltas = vec![
            create_delta(&["CREATE TABLE T1", "DROP VIEW V1"]),
            create_delta(&["ALTER TABLE T2 ADD C1"]),
        ];
        let original_deltas = deltas.clone();

        // Case 1: disabled_drop_types is None
        let result_none = with_disabled_drop_types_excluded(deltas.clone(), None);
        assert_eq!(
            result_none, original_deltas,
            "Should return original deltas when input is None"
        );

        // Case 2: disabled_drop_types is Some([])
        let result_empty = with_disabled_drop_types_excluded(deltas, Some(Vec::new()));
        assert_eq!(
            result_empty, original_deltas,
            "Should return original deltas when input is empty vec"
        );
    }

    #[test]
    fn test_basic_filtering_removes_matching_script() {
        let deltas = vec![create_delta(&[
            "CREATE TABLE T1",
            "DROP TABLE T1_OLD",
            "CREATE INDEX IDX",
        ])];
        let disabled = disabled_list(&["TABLE"]);

        let result = with_disabled_drop_types_excluded(deltas, disabled);

        assert_eq!(result.len(), 1, "Delta should not be skipped");
        assert_eq!(result[0].scripts.len(), 2, "One script should be removed");
        assert_eq!(result[0].scripts[0], "CREATE TABLE T1");
        assert_eq!(result[0].scripts[1], "CREATE INDEX IDX");
    }

    #[test]
    fn test_embedded_filtering_removes_alter_drop_column() {
        let deltas = vec![create_delta(&[
            "ALTER TABLE ORDERS ADD (NEW_COL NUMBER)",
            "ALTER TABLE CUSTOMERS DROP COLUMN OLD_COL", // Should be filtered
            "COMMIT",
        ])];
        let disabled = disabled_list(&["COLUMN"]);

        let result = with_disabled_drop_types_excluded(deltas, disabled);

        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].scripts.len(),
            2,
            "DROP COLUMN script should be removed"
        );
        assert_eq!(
            result[0].scripts[0],
            "ALTER TABLE ORDERS ADD (NEW_COL NUMBER)"
        );
        assert_eq!(result[0].scripts[1], "COMMIT");
    }

    #[test]
    fn test_case_insensitivity_is_enforced() {
        let deltas = vec![create_delta(&[
            "drop index IDX_1", // Lowercase DROP and TYPE
            "DROP VIEW V1",     // Uppercase
            "create OR REPLACE view v2",
        ])];
        let disabled = disabled_list(&["INDEX", "view"]); // Mixed case disabled types

        let result = with_disabled_drop_types_excluded(deltas, disabled);

        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].scripts.len(),
            1,
            "DROP INDEX and DROP VIEW should be removed"
        );
        assert_eq!(result[0].scripts[0], "create OR REPLACE view v2");
    }

    #[test]
    fn test_multiple_disabled_types_filtering() {
        let deltas = vec![create_delta(&[
            "DROP TABLE T1",      // Filtered (TABLE)
            "DROP SEQUENCE SEQ1", // Filtered (SEQUENCE)
            "CREATE TABLE T2",    // Kept
            "DROP TRIGGER TRG1",  // Not filtered
        ])];
        let disabled = disabled_list(&["TABLE", "SEQUENCE"]);

        let result = with_disabled_drop_types_excluded(deltas, disabled);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].scripts.len(), 2);
        assert_eq!(result[0].scripts[0], "CREATE TABLE T2");
        assert_eq!(result[0].scripts[1], "DROP TRIGGER TRG1");
    }

    #[test]
    fn test_delta_is_skipped_if_all_scripts_filtered() {
        let deltas = vec![
            create_delta(&["CREATE T1"]),                       // Delta 1: Kept
            create_delta(&["DROP TABLE T2"]),                   // Delta 2: Skipped entirely
            create_delta(&["DROP VIEW V3", "CREATE INDEX I3"]), // Delta 3: Script removed, delta kept
        ];
        let disabled = disabled_list(&["TABLE", "VIEW"]);

        let result = with_disabled_drop_types_excluded(deltas, disabled);

        assert_eq!(result.len(), 2, "Only two deltas should remain");

        // Delta 1 is untouched
        assert_eq!(result[0].scripts.len(), 1);
        assert_eq!(result[0].scripts[0], "CREATE T1");

        // Delta 3 has one script remaining
        assert_eq!(result[1].scripts.len(), 1);
        assert_eq!(result[1].scripts[0], "CREATE INDEX I3");
    }

    #[test]
    fn test_filtering_with_unrelated_text_is_ignored() {
        let deltas = vec![create_delta(&[
            "SELECT * FROM USERS WHERE STATUS = 'DROPPED'", // Should be kept (not DROP )
            "COMMENT ON TABLE T1 IS 'DROP ME LATER'",       // Should be kept
            "DROP USER U1",                                 // Filtered (if USER is disabled)
        ])];
        let disabled = disabled_list(&["USER", "TABLE"]);

        let result = with_disabled_drop_types_excluded(deltas, disabled);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].scripts.len(), 2);
        assert_eq!(
            result[0].scripts[0],
            "SELECT * FROM USERS WHERE STATUS = 'DROPPED'"
        );
        assert_eq!(
            result[0].scripts[1],
            "COMMENT ON TABLE T1 IS 'DROP ME LATER'"
        );
    }
}
