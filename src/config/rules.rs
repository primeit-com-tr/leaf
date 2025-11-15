use crate::utils::serde::deserialize_opt_vec_from_string;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RulesConfig {
    #[serde(default, deserialize_with = "deserialize_opt_vec_from_string")]
    pub exclude_object_types: Option<Vec<String>>,

    #[serde(default, deserialize_with = "deserialize_opt_vec_from_string")]
    pub exclude_object_names: Option<Vec<String>>,

    #[serde(
        default = "default_disabled_drop_types",
        deserialize_with = "deserialize_opt_vec_from_string"
    )]
    pub disabled_drop_types: Option<Vec<String>>,

    #[serde(default = "default_true")]
    pub disable_all_drops: bool,
}

fn default_true() -> bool {
    true
}

fn default_disabled_drop_types() -> Option<Vec<String>> {
    Some(vec![])
}

impl RulesConfig {
    pub fn combined_exclude_object_types(
        &self,
        exclude_object_types: Option<Vec<String>>,
    ) -> Option<Vec<String>> {
        let self_types_iter = self
            .exclude_object_types
            .as_deref()
            .unwrap_or_default()
            .iter();

        let param_types_iter = exclude_object_types.as_deref().unwrap_or_default().iter();

        let result: Vec<String> = param_types_iter
            .chain(self_types_iter)
            .cloned()
            .unique()
            .collect();

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    pub fn combined_exclude_object_names(
        &self,
        exclude_object_names: Option<Vec<String>>,
    ) -> Option<Vec<String>> {
        let param_names_iter = exclude_object_names.as_deref().unwrap_or_default().iter();

        let self_names_iter = self
            .exclude_object_names
            .as_deref()
            .unwrap_or_default()
            .iter();

        let result: Vec<String> = param_names_iter
            .chain(self_names_iter)
            .cloned()
            .unique()
            .collect();

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    pub fn combined_disabled_drop_types(
        &self,
        disabled_drop_types: Option<Vec<String>>,
    ) -> Option<Vec<String>> {
        let self_types_iter = self
            .disabled_drop_types
            .as_deref()
            .unwrap_or_default()
            .iter();

        let param_types_iter = disabled_drop_types.as_deref().unwrap_or_default().iter();

        let result: Vec<String> = param_types_iter
            .chain(self_types_iter)
            .cloned()
            .unique()
            .collect();

        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }
}

impl Default for RulesConfig {
    fn default() -> Self {
        Self {
            exclude_object_types: default_exclude_object_types(),
            exclude_object_names: None,
            disabled_drop_types: default_disabled_drop_types(),
            disable_all_drops: default_true(),
        }
    }
}

fn default_exclude_object_types() -> Option<Vec<String>> {
    Some(vec![
        "DATABASE LINK".to_string(),
        "INDEX PARTITION".to_string(),
        "JAVA CLASS".to_string(),
        "JAVA SOURCE".to_string(),
        "JOB".to_string(),
        "LIBRARY".to_string(),
        "SCHEDULE".to_string(),
        "SCHEDULE".to_string(),
        "SYNONYM".to_string(),
        "TABLE PARTITION".to_string(),
        "TABLE SUBPARTITION".to_string(),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a Vec<String> easily
    fn s_vec(items: &[&str]) -> Vec<String> {
        items.iter().map(|&s| s.to_string()).collect()
    }

    // Helper to create an Option<Vec<String>>
    fn s_opt_vec(items: &[&str]) -> Option<Vec<String>> {
        Some(s_vec(items))
    }

    // --- Tests for combined_exclude_object_types ---

    #[test]
    fn test_combined_types_merge_unique() {
        let config = RulesConfig {
            exclude_object_types: s_opt_vec(&["TYPE_A", "TYPE_B", "DUP_TYPE"]),
            exclude_object_names: None,
            ..Default::default()
        };

        let param = s_opt_vec(&["TYPE_C", "TYPE_D", "DUP_TYPE", "TYPE_B"]);

        let result = config.combined_exclude_object_types(param).unwrap();

        // FIX: Expected unique count is 5 (A, B, DUP_TYPE, C, D)
        assert_eq!(result.len(), 5);
        assert!(result.contains(&"TYPE_A".to_string()));
        assert!(result.contains(&"TYPE_B".to_string()));
        assert!(result.contains(&"TYPE_C".to_string()));
        assert!(result.contains(&"TYPE_D".to_string()));
        assert!(result.contains(&"DUP_TYPE".to_string())); // Add check for DUP_TYPE
    }

    #[test]
    fn test_combined_types_param_only() {
        let config = RulesConfig {
            exclude_object_types: None, // Config is None
            exclude_object_names: None,
            ..Default::default()
        };

        let param = s_opt_vec(&["TYPE_1", "TYPE_2"]);

        assert_eq!(
            config.combined_exclude_object_types(param),
            s_opt_vec(&["TYPE_1", "TYPE_2"])
        );
    }

    #[test]
    fn test_combined_types_self_only() {
        let config = RulesConfig {
            exclude_object_types: s_opt_vec(&["TYPE_X", "TYPE_Y"]), // Config has values
            exclude_object_names: None,
            ..Default::default()
        };

        let param: Option<Vec<String>> = None; // Parameter is None

        assert_eq!(
            config.combined_exclude_object_types(param),
            s_opt_vec(&["TYPE_X", "TYPE_Y"])
        );
    }

    #[test]
    fn test_combined_types_returns_none_when_empty() {
        let config = RulesConfig {
            exclude_object_types: None, // Config is None
            exclude_object_names: None,
            ..Default::default()
        };

        let param: Option<Vec<String>> = None; // Parameter is None

        // Should return None if both are None
        assert_eq!(config.combined_exclude_object_types(param), None);

        // Should return None if both are Some([])
        let config_empty = RulesConfig {
            exclude_object_types: Some(Vec::new()),
            exclude_object_names: None,
            ..Default::default()
        };
        assert_eq!(
            config_empty.combined_exclude_object_types(Some(Vec::new())),
            None
        );
    }

    // --- Tests for combined_exclude_object_names ---

    #[test]
    fn test_combined_names_merge_unique() {
        let config = RulesConfig {
            exclude_object_types: None,
            exclude_object_names: s_opt_vec(&["NAME_A", "NAME_B", "DUP_NAME"]),
            ..Default::default()
        };

        let param = s_opt_vec(&["NAME_C", "DUP_NAME", "NAME_B"]);

        let result = config.combined_exclude_object_names(param).unwrap();

        // FIX: Expected unique count is 4 (A, B, DUP_NAME, C)
        assert_eq!(result.len(), 4);
        assert!(result.contains(&"NAME_A".to_string()));
        assert!(result.contains(&"NAME_B".to_string()));
        assert!(result.contains(&"NAME_C".to_string()));
        assert!(result.contains(&"DUP_NAME".to_string())); // Add check for DUP_NAME
    }

    #[test]
    fn test_combined_names_param_only() {
        let config = RulesConfig {
            exclude_object_types: None,
            exclude_object_names: None, // Config is None
            ..Default::default()
        };

        let param = s_opt_vec(&["OBJ_1", "OBJ_2"]);

        assert_eq!(
            config.combined_exclude_object_names(param),
            s_opt_vec(&["OBJ_1", "OBJ_2"])
        );
    }

    // --- Tests for combined_disabled_drop_types ---

    #[test]
    fn test_combined_drop_types_merge_unique() {
        let config = RulesConfig {
            // Config has some disabled drop types
            disabled_drop_types: s_opt_vec(&["DROP_T1", "DROP_T2", "DUP_DROP"]),
            ..Default::default()
        };

        // Parameter has new types and duplicates
        let param = s_opt_vec(&["DROP_T3", "DUP_DROP", "DROP_T2"]);

        let result = config.combined_disabled_drop_types(param).unwrap();

        // Expected unique count is 4 (T1, T2, T3, DUP_DROP)
        assert_eq!(result.len(), 4);
        assert!(result.contains(&"DROP_T1".to_string()));
        assert!(result.contains(&"DROP_T2".to_string()));
        assert!(result.contains(&"DROP_T3".to_string()));
        assert!(result.contains(&"DUP_DROP".to_string()));
    }

    #[test]
    fn test_combined_drop_types_param_only() {
        // Config defaults to Some([]) because of default_disabled_drop_types()
        let config = RulesConfig {
            disabled_drop_types: None,
            ..Default::default()
        };

        let param = s_opt_vec(&["ONLY_P1", "ONLY_P2"]);

        // Should return the parameter values
        assert_eq!(
            config.combined_disabled_drop_types(param),
            s_opt_vec(&["ONLY_P1", "ONLY_P2"])
        );
    }

    #[test]
    fn test_combined_drop_types_self_only() {
        let config = RulesConfig {
            // Config has values
            disabled_drop_types: s_opt_vec(&["ONLY_S1", "ONLY_S2"]),
            ..Default::default()
        };

        let param: Option<Vec<String>> = None; // Parameter is None

        // Should return the config values
        assert_eq!(
            config.combined_disabled_drop_types(param),
            s_opt_vec(&["ONLY_S1", "ONLY_S2"])
        );
    }

    #[test]
    fn test_combined_drop_types_returns_none_when_empty() {
        // Both self and param are None (or Some([])), should result in None
        let config_none = RulesConfig {
            disabled_drop_types: None,
            ..Default::default()
        };

        assert_eq!(
            config_none.combined_disabled_drop_types(None),
            None,
            "Should return None when both are None"
        );

        // Both self and param are Some([])
        let config_empty = RulesConfig {
            disabled_drop_types: Some(Vec::new()),
            ..Default::default()
        };

        assert_eq!(
            config_empty.combined_disabled_drop_types(Some(Vec::new())),
            None,
            "Should return None when both are Some([])"
        );
    }

    // --- Tests for Default Implementation ---

    #[test]
    fn test_default_implementation() {
        let default_config = RulesConfig::default();

        // Test exclude_object_types defaults to the predefined list
        let expected_types = default_exclude_object_types().unwrap();

        // The default implementation contains a duplicate "SCHEDULE", so we collect
        // the default list and check against the unique expected values.
        let unique_expected_types: Vec<String> = expected_types.iter().cloned().unique().collect();

        // Check if the actual default is the unique set of the expected types
        // Note: The `default_exclude_object_types` function itself returns a list
        // with a duplicate, but the user likely intends this as the source list.
        assert_eq!(
            default_config.exclude_object_types.as_ref().unwrap().len(),
            11
        ); // The raw length (including duplicate)

        // Asserting the content against the unique list is more robust if we assume the source *should* be unique
        let default_types_unique: Vec<String> = default_config
            .exclude_object_types
            .as_ref()
            .unwrap()
            .iter()
            .cloned()
            .unique()
            .collect();
        assert_eq!(default_types_unique.len(), 10);
        assert_eq!(default_types_unique, unique_expected_types);

        // Test exclude_object_names defaults to None
        assert_eq!(default_config.exclude_object_names, None);
    }

    #[test]
    fn test_default_exclude_object_types_has_duplicate() {
        let types = default_exclude_object_types().unwrap();
        // Check that the raw list has a length of 11 (including the "SCHEDULE" duplicate)
        assert_eq!(types.len(), 11);

        // Check that after applying unique, the length is 10
        let unique_types: Vec<String> = types.iter().cloned().unique().collect();
        assert_eq!(unique_types.len(), 10);
    }
}
