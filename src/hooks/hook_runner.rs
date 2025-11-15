use tera::Context;

use crate::{oracle::OracleClient, types::Hooks};
use anyhow::Result;

pub struct HookRunnerContext<F>
where
    F: FnMut(String),
{
    tera_ctx: Context,
    progress: F,
}

impl<F> HookRunnerContext<F>
where
    F: FnMut(String),
{
    pub fn new(tera_ctx: Context, progress: F) -> Self {
        Self { tera_ctx, progress }
    }

    pub fn progress(&mut self, msg: String) {
        (self.progress)(msg)
    }

    pub fn tera_ctx(&self) -> &Context {
        &self.tera_ctx
    }
}

pub struct HookRunner<F>
where
    F: FnMut(String),
{
    disable_hooks: bool,
    hooks: Option<Hooks>,
    ctx: HookRunnerContext<F>,
}

impl<F> HookRunner<F>
where
    F: FnMut(String),
{
    pub fn new(disable_hooks: bool, hooks: Option<Hooks>, ctx: HookRunnerContext<F>) -> Self {
        Self {
            disable_hooks,
            hooks,
            ctx,
        }
    }

    async fn run(&mut self, client: &OracleClient, scripts: Vec<String>) -> Result<()> {
        if self.disable_hooks {
            return Ok(());
        }
        let script_count = scripts.len();

        if script_count == 0 {
            self.ctx.progress("✅ Hooks are empty".to_string());
            return Ok(());
        }

        for (i, script) in scripts.into_iter().enumerate() {
            if script.trim().is_empty() {
                self.ctx.progress(format!("Skipping empty hook {}", i + 1));
                continue;
            }
            self.ctx
                .progress(format!("Executing hook {} of {}", i + 1, script_count));

            let script = script.trim();
            if script.is_empty() {
                continue;
            }
            client.execute(&script).await?;
        }

        Ok(())
    }

    pub async fn run_pre_prepare_deployment(&mut self, client: &OracleClient) -> Result<()> {
        if self.disable_hooks {
            return Ok(());
        }

        if self.hooks.is_none() {
            self.ctx
                .progress("✅ No pre-prepare-deployment hooks found".to_string());
            return Ok(());
        }
        let hooks = self.hooks.as_ref().unwrap();

        self.ctx
            .progress("✅ Executing pre-prepare-deployment hooks".to_string());

        self.run(
            client,
            hooks
                .get_pre_prepare_deployment(&self.ctx.tera_ctx())?
                .unwrap_or_default(),
        )
        .await
    }

    pub async fn run_post_prepare_deployment(&mut self, client: &OracleClient) -> Result<()> {
        if self.disable_hooks {
            return Ok(());
        }
        if self.hooks.is_none() {
            self.ctx
                .progress("✅ No post-prepare-deployment hooks found".to_string());
            return Ok(());
        }
        let hooks = self.hooks.as_ref().unwrap();

        self.ctx
            .progress("✅ Executing post-prepare-deployment hooks".to_string());

        self.run(
            client,
            hooks
                .get_post_prepare_deployment(&self.ctx.tera_ctx())?
                .unwrap_or_default(),
        )
        .await
    }

    pub async fn run_pre_apply_deployment(&mut self, client: &OracleClient) -> Result<()> {
        if self.disable_hooks {
            return Ok(());
        }
        if self.hooks.is_none() {
            self.ctx
                .progress("✅ No pre-apply-deployment hooks found".to_string());
            return Ok(());
        }
        let hooks = self.hooks.as_ref().unwrap();

        self.ctx
            .progress("✅ Executing pre-apply-deployment hooks".to_string());

        self.run(
            client,
            hooks
                .get_pre_apply_deployment(&self.ctx.tera_ctx())?
                .unwrap_or_default(),
        )
        .await
    }

    pub async fn run_post_apply_deployment(&mut self, client: &OracleClient) -> Result<()> {
        if self.disable_hooks {
            return Ok(());
        }

        if self.hooks.is_none() {
            self.ctx
                .progress("✅ No post-apply-deployment hooks found".to_string());
            return Ok(());
        }
        let hooks = self.hooks.as_ref().unwrap();

        self.ctx
            .progress("✅ Executing post-apply-deployment hooks".to_string());

        self.run(
            client,
            hooks
                .get_post_apply_deployment(&self.ctx.tera_ctx())?
                .unwrap_or_default(),
        )
        .await
    }
}
