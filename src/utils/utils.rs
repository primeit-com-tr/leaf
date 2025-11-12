use std::collections::HashMap;

use crate::types::Object;

pub fn format_sql_list(items: Vec<String>) -> String {
    items
        .iter()
        .map(|item| format!("'{}'", item))
        .collect::<Vec<String>>()
        .join(",")
}

pub fn objects_as_map(objects: Vec<Object>) -> HashMap<(String, String, String), Object> {
    let res: HashMap<(String, String, String), Object> = objects
        .iter()
        .map(|obj| {
            (
                (
                    obj.owner.clone(),
                    obj.object_name.clone(),
                    obj.object_type.clone(),
                ),
                obj.clone(),
            )
        })
        .collect();
    res
}

pub fn indent_lines(s: &str, spaces: usize) -> String {
    let indent = " ".repeat(spaces);
    s.lines()
        .map(|line| format!("{}{}", indent, line))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    fn mock_object(owner: &str, name: &str, obj_type: &str) -> Object {
        Object {
            owner: owner.to_string(),
            object_name: name.to_string(),
            object_type: obj_type.to_string(),
            last_ddl_time: NaiveDateTime::parse_from_str(
                "2025-11-07 12:00:00",
                "%Y-%m-%d %H:%M:%S",
            )
            .unwrap(),
            ddl: Some("CREATE TABLE test (id INT)".to_string()),
        }
    }

    #[test]
    fn test_format_sql_list_single() {
        let result = format_sql_list(vec!["foo".to_string()]);
        assert_eq!(result, "'foo'");
    }

    #[test]
    fn test_format_sql_list_multiple() {
        let result = format_sql_list(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        assert_eq!(result, "'a','b','c'");
    }

    #[test]
    fn test_format_sql_list_empty() {
        let result = format_sql_list(vec![]);
        assert_eq!(result, "");
    }

    #[test]
    fn test_objects_as_map_basic() {
        let obj1 = mock_object("SCOTT", "EMP", "TABLE");
        let obj2 = mock_object("HR", "DEPT", "VIEW");

        let map = objects_as_map(vec![obj1.clone(), obj2.clone()]);

        assert_eq!(map.len(), 2);
        assert_eq!(
            map.get(&("SCOTT".to_string(), "EMP".to_string(), "TABLE".to_string()))
                .unwrap()
                .object_name,
            "EMP"
        );
        assert_eq!(
            map.get(&("HR".to_string(), "DEPT".to_string(), "VIEW".to_string()))
                .unwrap()
                .object_name,
            "DEPT"
        );
    }

    #[test]
    fn test_objects_as_map_duplicate_keys_last_wins() {
        let obj1 = mock_object("SYS", "USERS", "TABLE");
        let mut obj2 = obj1.clone();
        obj2.ddl = Some("ALTER TABLE USERS ADD COLUMN name VARCHAR(100)".to_string());

        let map = objects_as_map(vec![obj1.clone(), obj2.clone()]);
        let key = ("SYS".to_string(), "USERS".to_string(), "TABLE".to_string());
        assert_eq!(map.len(), 1);
        assert_eq!(
            map.get(&key).unwrap().ddl.as_ref().unwrap(),
            obj2.ddl.as_ref().unwrap()
        );
    }

    #[test]
    fn test_indent_lines_basic() {
        let input = "line1\nline2";
        let result = indent_lines(input, 2);
        assert_eq!(result, "  line1\n  line2");
    }

    #[test]
    fn test_indent_lines_empty_string() {
        let input = "";
        let result = indent_lines(input, 4);
        assert_eq!(result, "");
    }

    #[test]
    fn test_indent_lines_single_line() {
        let input = "onlyone";
        let result = indent_lines(input, 3);
        assert_eq!(result, "   onlyone");
    }
}
