use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HooksConfig {
    #[serde(default)]
    pub pre_plan_run: Option<Vec<String>>,

    #[serde(default)]
    pub post_plan_run: Option<Vec<String>>,

    #[serde(default)]
    pub pre_prepare_deployment: Option<Vec<String>>,

    #[serde(default)]
    pub post_prepare_deployment: Option<Vec<String>>,

    #[serde(default)]
    pub pre_apply_deployment: Option<Vec<String>>,

    #[serde(default)]
    pub post_apply_deployment: Option<Vec<String>>,
}

impl Default for HooksConfig {
    fn default() -> Self {
        Self {
            pre_plan_run: None,
            post_plan_run: None,
            pre_apply_deployment: None,
            post_apply_deployment: None,
            pre_prepare_deployment: None,
            post_prepare_deployment: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn default_is_all_none() {
        let cfg = HooksConfig::default();

        assert!(cfg.pre_plan_run.is_none());
        assert!(cfg.post_plan_run.is_none());
        assert!(cfg.pre_prepare_deployment.is_none());
        assert!(cfg.post_prepare_deployment.is_none());
        assert!(cfg.pre_apply_deployment.is_none());
        assert!(cfg.post_apply_deployment.is_none());
    }

    #[test]
    fn deserialize_empty_object_gives_default() {
        let json = "{}";
        let cfg: HooksConfig = serde_json::from_str(json).unwrap();

        assert!(cfg.pre_plan_run.is_none());
        assert!(cfg.post_plan_run.is_none());
        assert!(cfg.pre_prepare_deployment.is_none());
        assert!(cfg.post_prepare_deployment.is_none());
        assert!(cfg.pre_apply_deployment.is_none());
        assert!(cfg.post_apply_deployment.is_none());
    }

    #[test]
    fn deserialize_full_config() {
        let json = r#"
        {
            "pre_plan_run": ["echo prerun"],
            "post_plan_run": ["echo postrun"],
            "pre_prepare_deployment": ["cmd a", "cmd b"],
            "post_prepare_deployment": ["cmd c"],
            "pre_apply_deployment": ["before"],
            "post_apply_deployment": ["after"]
        }
        "#;

        let cfg: HooksConfig = serde_json::from_str(json).unwrap();

        assert_eq!(cfg.pre_plan_run, Some(vec!["echo prerun".into()]));
        assert_eq!(cfg.post_plan_run, Some(vec!["echo postrun".into()]));
        assert_eq!(
            cfg.pre_prepare_deployment,
            Some(vec!["cmd a".into(), "cmd b".into()])
        );
        assert_eq!(cfg.post_prepare_deployment, Some(vec!["cmd c".into()]));
        assert_eq!(cfg.pre_apply_deployment, Some(vec!["before".into()]));
        assert_eq!(cfg.post_apply_deployment, Some(vec!["after".into()]));
    }

    #[test]
    fn roundtrip_serde() {
        let original = HooksConfig {
            pre_plan_run: Some(vec!["a".into()]),
            post_plan_run: Some(vec!["b".into()]),
            pre_prepare_deployment: None,
            post_prepare_deployment: Some(vec!["x".into()]),
            pre_apply_deployment: None,
            post_apply_deployment: Some(vec!["y".into()]),
        };

        let json = serde_json::to_string(&original).unwrap();
        let decoded: HooksConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.pre_plan_run, Some(vec!["a".into()]));
        assert_eq!(decoded.post_plan_run, Some(vec!["b".into()]));
        assert_eq!(decoded.post_prepare_deployment, Some(vec!["x".into()]));
        assert_eq!(decoded.post_apply_deployment, Some(vec!["y".into()]));
        assert!(decoded.pre_prepare_deployment.is_none());
        assert!(decoded.pre_apply_deployment.is_none());
    }

    #[test]
    fn missing_fields_still_default_to_none() {
        let json = r#"
        {
            "pre_plan_run": ["hello"]
        }
        "#;

        let cfg: HooksConfig = serde_json::from_str(json).unwrap();

        assert_eq!(cfg.pre_plan_run, Some(vec!["hello".into()]));
        assert!(cfg.post_plan_run.is_none());
        assert!(cfg.pre_prepare_deployment.is_none());
        assert!(cfg.post_prepare_deployment.is_none());
        assert!(cfg.pre_apply_deployment.is_none());
        assert!(cfg.post_apply_deployment.is_none());
    }
}
