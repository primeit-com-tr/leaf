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

pub fn find_deltas(
    sources: Vec<Object>,
    targets: Vec<Object>,
    disable_all_drops: bool,
) -> Vec<Delta> {
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

    if disable_all_drops {
        return with_disabled_drop_types_excluded(deltas, Some(vec!["COLUMN".to_string()]));
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
        let mut targets = sources.clone();
        for t in &mut targets {
            t.ddl = Some("-- old DDL".to_string());
        }

        let deltas = find_deltas(sources.clone(), targets, false);

        for (i, delta) in deltas.iter().enumerate() {
            let source_ddl = sources[i].ddl.as_ref().unwrap();
            assert_eq!(delta.scripts, vec![source_ddl.clone()]);
        }
    }

    #[test]
    fn test_find_deltas_object_deletion() {
        let sources = vec![];
        let targets = mock_objects();

        let deltas = find_deltas(sources, targets.clone(), false);

        for (i, delta) in deltas.iter().enumerate() {
            let target = &targets[i];
            let expected = format!(
                "DROP {} {}.{}",
                target.object_type, target.owner, target.object_name
            );
            assert!(delta.scripts.contains(&expected));
        }
    }

    // ... (keep all other tests, just add false parameter to find_deltas calls)

    #[test]
    fn test_find_deltas_add_table() {
        let s = mock_object("HR", "EMP", "TABLE", "CREATE TABLE EMP (ID INT)");
        let deltas = find_deltas(vec![s.clone()], vec![], false);

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
        let deltas = find_deltas(vec![], vec![t.clone()], false);

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
        let deltas = find_deltas(vec![s.clone()], vec![t.clone()], false);

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

        let deltas = find_deltas(vec![s.clone()], vec![t.clone()], false);

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

        let deltas = find_deltas(vec![s], vec![t], false);

        assert_eq!(deltas.len(), 1);
        let delta = &deltas[0];
        assert!(delta.scripts.is_empty());
        assert!(delta.rollback_scripts.is_empty());
    }

    #[test]
    fn test_find_deltas_multiple_objects() {
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

        let deltas = find_deltas(vec![s1, s2], vec![t1, t2], false);

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

        let deltas = find_deltas(vec![s1, s2], vec![], false);

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
        let deltas = find_deltas(vec![], vec![], false);
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

        let deltas = find_deltas(vec![s], vec![t], false);

        assert_eq!(deltas.len(), 1);
        let delta = &deltas[0];
        assert_eq!(delta.object_type, "VIEW");
        assert!(delta.scripts.iter().any(|s| s.contains("CREATE VIEW")));
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

        let deltas = find_deltas(vec![s.clone()], vec![t.clone()], false);

        let delta = &deltas[0];
        assert_eq!(delta.source_ddl_time, Some(s.last_ddl_time));
        assert_eq!(delta.target_ddl_time, Some(t.last_ddl_time));
    }

    #[test]
    fn test_find_deltas_complex_scenario() {
        let sources = vec![
            mock_object(
                "HR",
                "EMPLOYEES",
                "TABLE",
                r#"CREATE TABLE "EMPLOYEES" ("ID" NUMBER, "NAME" VARCHAR2(200), "DEPT_ID" NUMBER)"#,
            ),
            mock_object(
                "HR",
                "DEPARTMENTS",
                "TABLE",
                r#"CREATE TABLE "DEPARTMENTS" ("ID" NUMBER, "NAME" VARCHAR2(100))"#,
            ),
            mock_object(
                "HR",
                "LOCATIONS",
                "TABLE",
                r#"CREATE TABLE "LOCATIONS" ("ID" NUMBER)"#,
            ),
            mock_object(
                "HR",
                "EMP_VIEW",
                "VIEW",
                "CREATE VIEW EMP_VIEW AS SELECT ID, NAME FROM EMPLOYEES",
            ),
        ];

        let targets = vec![
            mock_object(
                "HR",
                "EMPLOYEES",
                "TABLE",
                r#"CREATE TABLE "EMPLOYEES" ("ID" NUMBER, "NAME" VARCHAR2(100))"#,
            ),
            mock_object(
                "HR",
                "SALARIES",
                "TABLE",
                r#"CREATE TABLE "SALARIES" ("EMP_ID" NUMBER, "AMOUNT" NUMBER)"#,
            ),
            mock_object(
                "HR",
                "LOCATIONS",
                "TABLE",
                r#"CREATE TABLE "LOCATIONS" ("ID" NUMBER)"#,
            ),
            mock_object(
                "HR",
                "EMP_VIEW",
                "VIEW",
                "CREATE VIEW EMP_VIEW AS SELECT ID FROM EMPLOYEES",
            ),
        ];

        let deltas = find_deltas(sources, targets, false);

        assert_eq!(deltas.len(), 5);

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

        let dept = deltas
            .iter()
            .find(|d| d.object_name == "DEPARTMENTS")
            .unwrap();
        assert!(dept.scripts.iter().any(|s| s.contains("CREATE TABLE")));
        assert!(dept.source_ddl.is_some());
        assert!(dept.target_ddl.is_none() || dept.target_ddl.as_ref().unwrap().is_empty());

        let sal = deltas.iter().find(|d| d.object_name == "SALARIES").unwrap();
        assert!(sal.scripts.iter().any(|s| s.contains("DROP TABLE")));
        assert!(sal.source_ddl.is_none());
        assert!(sal.target_ddl.is_some());

        let loc = deltas
            .iter()
            .find(|d| d.object_name == "LOCATIONS")
            .unwrap();
        assert!(loc.scripts.is_empty());

        let view = deltas.iter().find(|d| d.object_name == "EMP_VIEW").unwrap();
        assert!(view.scripts.iter().any(|s| s.contains("CREATE VIEW")));
    }

    // ==================== Tests with disable_all_drops = true ====================

    #[test]
    fn test_find_deltas_with_disable_all_drops_filters_drop_columns() {
        let s = mock_object("HR", "EMP", "TABLE", r#"CREATE TABLE "EMP" ("ID" NUMBER)"#);
        let t = mock_object(
            "HR",
            "EMP",
            "TABLE",
            r#"CREATE TABLE "EMP" ("ID" NUMBER, "NAME" VARCHAR2(100))"#,
        );

        let deltas = find_deltas(vec![s.clone()], vec![t.clone()], true);

        // Delta should be filtered out entirely since only DROP COLUMN would remain
        assert_eq!(
            deltas.len(),
            0,
            "Delta with only DROP COLUMN should be filtered out"
        );
    }

    #[test]
    fn test_find_deltas_with_disable_all_drops_filters_multiple_drop_columns() {
        let s = mock_object("HR", "EMP", "TABLE", r#"CREATE TABLE "EMP" ("ID" NUMBER)"#);
        let t = mock_object(
            "HR",
            "EMP",
            "TABLE",
            r#"CREATE TABLE "EMP" ("ID" NUMBER, "NAME" VARCHAR2(100), "EMAIL" VARCHAR2(200))"#,
        );

        let deltas = find_deltas(vec![s], vec![t], true);

        // All scripts would be DROP COLUMN, so delta should be filtered
        assert_eq!(deltas.len(), 0);
    }

    #[test]
    fn test_find_deltas_with_disable_all_drops_keeps_add_and_modify() {
        let s = mock_object(
            "HR",
            "EMP",
            "TABLE",
            r#"CREATE TABLE "EMP" ("ID" NUMBER, "NAME" VARCHAR2(200), "AGE" NUMBER)"#,
        );
        let t = mock_object(
            "HR",
            "EMP",
            "TABLE",
            r#"CREATE TABLE "EMP" ("ID" NUMBER, "NAME" VARCHAR2(100), "EMAIL" VARCHAR2(100))"#,
        );

        let deltas = find_deltas(vec![s], vec![t], true);

        assert_eq!(deltas.len(), 1);
        let delta = &deltas[0];

        // Should contain ADD for AGE
        assert!(
            delta
                .scripts
                .iter()
                .any(|s| s.contains("ADD") && s.contains("AGE"))
        );

        // Should contain MODIFY for NAME
        assert!(
            delta
                .scripts
                .iter()
                .any(|s| s.contains("MODIFY") && s.contains("NAME"))
        );

        // Should NOT contain DROP COLUMN for EMAIL
        assert!(!delta.scripts.iter().any(|s| s.contains("DROP COLUMN")));
    }

    #[test]
    fn test_find_deltas_with_disable_all_drops_no_object_drops() {
        let s = mock_object("HR", "EMP", "TABLE", "CREATE TABLE EMP (ID INT)");
        let t = mock_object("HR", "DEPT", "TABLE", "CREATE TABLE DEPT (ID INT)");

        let deltas = find_deltas(vec![s], vec![t], true);

        // EMP should be added
        let emp_delta = deltas.iter().find(|d| d.object_name == "EMP");
        assert!(emp_delta.is_some());

        // DEPT should NOT appear (would only have DROP TABLE)
        // But wait, disable_all_drops only returns early with COLUMN filtering for sources
        // Targets are not processed when disable_all_drops is true!
        let dept_delta = deltas.iter().find(|d| d.object_name == "DEPT");
        assert!(
            dept_delta.is_none(),
            "DEPT should not appear since targets are not processed"
        );
    }

    #[test]
    fn test_find_deltas_with_disable_all_drops_mixed_operations() {
        let s1 = mock_object(
            "HR",
            "EMP",
            "TABLE",
            r#"CREATE TABLE "EMP" ("ID" NUMBER, "NAME" VARCHAR2(100), "SALARY" NUMBER)"#,
        );
        let s2 = mock_object(
            "HR",
            "DEPT",
            "TABLE",
            r#"CREATE TABLE "DEPT" ("ID" NUMBER)"#,
        );

        let t1 = mock_object(
            "HR",
            "EMP",
            "TABLE",
            r#"CREATE TABLE "EMP" ("ID" NUMBER, "NAME" VARCHAR2(200), "EMAIL" VARCHAR2(100))"#,
        );

        let deltas = find_deltas(vec![s1, s2], vec![t1], true);

        // EMP: has ADD SALARY, MODIFY NAME, and DROP EMAIL (should filter DROP)
        let emp = deltas.iter().find(|d| d.object_name == "EMP").unwrap();
        assert!(
            emp.scripts
                .iter()
                .any(|s| s.contains("ADD") && s.contains("SALARY"))
        );
        assert!(
            emp.scripts
                .iter()
                .any(|s| s.contains("MODIFY") && s.contains("NAME"))
        );
        assert!(!emp.scripts.iter().any(|s| s.contains("DROP COLUMN")));

        // DEPT: new table, should be present
        let dept = deltas.iter().find(|d| d.object_name == "DEPT");
        assert!(dept.is_some());
    }

    #[test]
    fn test_find_deltas_with_disable_all_drops_only_drop_columns() {
        let s = mock_object("HR", "EMP", "TABLE", r#"CREATE TABLE "EMP" ("ID" NUMBER)"#);
        let t = mock_object(
            "HR",
            "EMP",
            "TABLE",
            r#"CREATE TABLE "EMP" ("ID" NUMBER, "NAME" VARCHAR2(100), "EMAIL" VARCHAR2(200), "PHONE" VARCHAR2(20))"#,
        );

        let deltas = find_deltas(vec![s], vec![t], true);

        // All operations are DROP COLUMN, so delta should be completely filtered
        assert_eq!(
            deltas.len(),
            0,
            "Delta should be filtered when all scripts are DROP COLUMN"
        );
    }
}
