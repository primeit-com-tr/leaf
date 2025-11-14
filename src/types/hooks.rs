use serde::{Deserialize, Serialize};

use crate::config::HooksConfig;
use anyhow::{Context as _, Result};
use tera::{Context, Tera};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Hooks {
    pub pre_plan_run: Option<Vec<String>>,
    pub post_plan_run: Option<Vec<String>>,
    pub pre_prepare_deployment: Option<Vec<String>>,
    pub post_prepare_deployment: Option<Vec<String>>,
    pub pre_apply_deployment: Option<Vec<String>>,
    pub post_apply_deployment: Option<Vec<String>>,
}

impl Hooks {
    pub fn from_config(config: HooksConfig) -> Self {
        Self {
            pre_plan_run: config.pre_plan_run,
            post_plan_run: config.post_plan_run,
            pre_prepare_deployment: config.pre_prepare_deployment,
            post_prepare_deployment: config.post_prepare_deployment,
            pre_apply_deployment: config.pre_apply_deployment,
            post_apply_deployment: config.post_apply_deployment,
        }
    }

    fn render_hooks(
        &self,
        ctx: &Context,
        hooks: &Option<Vec<String>>,
    ) -> Result<Option<Vec<String>>> {
        let mut t = Tera::default();

        hooks
            .as_ref()
            .map(|scripts| {
                scripts
                    .iter()
                    .map(|s| {
                        t.render_str(s, ctx)
                            .context("Failed to render pre_plan_run hook script")
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()
    }

    pub fn get_pre_plan_run(&self, ctx: &Context) -> Result<Option<Vec<String>>> {
        self.render_hooks(ctx, &self.pre_plan_run)
    }

    pub fn get_post_plan_run(&self, ctx: &Context) -> Result<Option<Vec<String>>> {
        self.render_hooks(ctx, &self.post_plan_run)
    }

    pub fn get_pre_prepare_deployment(&self, ctx: &Context) -> Result<Option<Vec<String>>> {
        self.render_hooks(ctx, &self.pre_prepare_deployment)
    }

    pub fn get_post_prepare_deployment(&self, ctx: &Context) -> Result<Option<Vec<String>>> {
        self.render_hooks(ctx, &self.post_prepare_deployment)
    }

    pub fn get_pre_apply_deployment(&self, ctx: &Context) -> Result<Option<Vec<String>>> {
        self.render_hooks(ctx, &self.pre_apply_deployment)
    }

    pub fn get_post_apply_deployment(&self, ctx: &Context) -> Result<Option<Vec<String>>> {
        self.render_hooks(ctx, &self.post_apply_deployment)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tera::Context;

    #[test]
    fn default_hooks_are_none() {
        let hooks = Hooks::default();

        assert!(hooks.pre_plan_run.is_none());
        assert!(hooks.post_plan_run.is_none());
        assert!(hooks.pre_prepare_deployment.is_none());
        assert!(hooks.post_prepare_deployment.is_none());
        assert!(hooks.pre_apply_deployment.is_none());
        assert!(hooks.post_apply_deployment.is_none());
    }

    #[test]
    fn from_config_transfers_values() {
        let cfg = crate::config::HooksConfig {
            pre_plan_run: Some(vec!["echo pre".to_string()]),
            post_plan_run: None,
            pre_prepare_deployment: Some(vec!["prepare".to_string()]),
            post_prepare_deployment: None,
            pre_apply_deployment: None,
            post_apply_deployment: Some(vec!["apply_done".to_string()]),
        };

        let hooks = Hooks::from_config(cfg.clone());

        assert_eq!(hooks.pre_plan_run, cfg.pre_plan_run);
        assert_eq!(hooks.post_plan_run, cfg.post_plan_run);
        assert_eq!(hooks.pre_prepare_deployment, cfg.pre_prepare_deployment);
        assert_eq!(hooks.post_prepare_deployment, cfg.post_prepare_deployment);
        assert_eq!(hooks.pre_apply_deployment, cfg.pre_apply_deployment);
        assert_eq!(hooks.post_apply_deployment, cfg.post_apply_deployment);
    }

    #[test]
    fn render_hooks_returns_none_for_none() {
        let hooks = Hooks::default();
        let ctx = Context::new();

        let rendered = hooks.get_pre_plan_run(&ctx).unwrap();
        assert!(rendered.is_none());
    }

    #[test]
    fn render_hooks_renders_templates() {
        let hooks = Hooks {
            pre_plan_run: Some(vec!["Hello {{ name }}".into()]),
            ..Default::default()
        };

        let mut ctx = Context::new();
        ctx.insert("name", "Alice");

        let rendered = hooks.get_pre_plan_run(&ctx).unwrap();
        assert_eq!(rendered, Some(vec!["Hello Alice".to_string()]));
    }

    #[test]
    fn render_hooks_handles_multiple_scripts() {
        let hooks = Hooks {
            pre_plan_run: Some(vec!["Hello {{ name }}".into(), "Goodbye {{ name }}".into()]),
            ..Default::default()
        };

        let mut ctx = Context::new();
        ctx.insert("name", "Bob");

        let rendered = hooks.get_pre_plan_run(&ctx).unwrap();
        assert_eq!(
            rendered,
            Some(vec!["Hello Bob".to_string(), "Goodbye Bob".to_string()])
        );
    }

    #[test]
    fn render_hooks_fails_on_invalid_template() {
        let hooks = Hooks {
            pre_plan_run: Some(vec!["{{ unclosed".into()]),
            ..Default::default()
        };

        let ctx = Context::new();

        let result = hooks.get_pre_plan_run(&ctx);
        assert!(result.is_err());
    }
}
