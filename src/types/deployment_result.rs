use chrono::NaiveDateTime;

use crate::{
    entities::{deployment::Model as DeploymentModel, plan::Model as PlanModel},
    types::{ChangesetStatus, Delta, DeploymentStatus},
    utils::{format_duration, indent_lines},
};

#[derive(Debug, Clone, Default)]
pub struct DryDeployment {
    pub plan: PlanModel,
    pub deltas: Vec<Delta>,
}

#[derive(Debug, Clone)]
pub enum DeploymentResultType {
    Deployment(DeploymentModel),
    DryDeployment(DryDeployment),
}

impl std::fmt::Display for DeploymentResultType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeploymentResultType::Deployment(deployment) => {
                writeln!(f, "ID: {}", deployment.id)?;
                writeln!(f, "Plan ID: {}", deployment.plan_id)?;
                writeln!(f, "Status: {}", deployment.status)?;
                writeln!(f, "Created at: {}", deployment.created_at)?;
                writeln!(
                    f,
                    "Started at: {}",
                    deployment.started_at.unwrap_or_default()
                )?;
                writeln!(f, "Ended at: {}", deployment.ended_at.unwrap_or_default())?;
                writeln!(
                    f,
                    "Duration: {}",
                    format_duration(
                        deployment.started_at.map(|dt| dt.and_utc()),
                        deployment.ended_at.map(|dt| dt.and_utc())
                    )
                )?;

                Ok(())
            }
            DeploymentResultType::DryDeployment(dry) => {
                writeln!(f, "Total Deltas: {}", dry.deltas.len())?;
                for (i, delta) in dry.deltas.iter().enumerate() {
                    writeln!(
                        f,
                        "\n{}. {} {}.{}",
                        i + 1,
                        delta.object_type,
                        delta.object_owner,
                        delta.object_name
                    )?;
                    if !delta.scripts.is_empty() {
                        writeln!(f, "   📝 Scripts: {} lines", delta.scripts.len())?;
                    }
                    if !delta.rollback_scripts.is_empty() {
                        writeln!(f, "   ↩️ Rollback: {} lines", delta.rollback_scripts.len())?;
                    }
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DeploymentItem {
    pub object_type: String,
    pub object_name: String,
    pub object_owner: String,
    pub source_ddl: Option<String>,
    pub target_ddl: Option<String>,
    pub scripts: Vec<String>,
    pub rollback_scripts: Vec<String>,
    pub status: Option<ChangesetStatus>,
    pub errors: Option<Vec<String>>,
}

impl std::fmt::Display for DeploymentItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "{} {}.{}",
            self.object_type, self.object_owner, self.object_name
        )?;
        writeln!(
            f,
            "Source ddl lines: {}",
            self.source_ddl
                .as_ref()
                .map_or(0, |ddl| ddl.lines().count())
        )?;
        writeln!(
            f,
            "Target ddl lines: {}",
            self.target_ddl
                .as_ref()
                .map_or(0, |ddl| ddl.lines().count())
        )?;
        writeln!(f, "Scripts lines: {}", self.scripts.len())?;
        writeln!(f, "Rollback scripts lines: {}", self.rollback_scripts.len())?;
        if let Some(status) = &self.status {
            writeln!(f, "Status: {}", status)?;
        }

        if let Some(errors) = &self.errors
            && !errors.is_empty()
        {
            writeln!(f, "Errors:")?;
            for error in errors {
                writeln!(f, " - {}", error)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct DeploymentResultDetails {
    pub is_dry_run: bool,
    pub id: Option<i32>,
    pub plan_id: i32,
    pub plan_name: String,
    pub source_connection_id: i32,
    pub source_connection_name: String,
    pub target_connection_id: i32,
    pub target_connection_name: String,
    pub status: Option<DeploymentStatus>,
    pub started_at: Option<NaiveDateTime>,
    pub ended_at: Option<NaiveDateTime>,
    pub items: Vec<DeploymentItem>,
}

impl std::fmt::Display for DeploymentResultDetails {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.is_dry_run && self.id.is_some() {
            writeln!(f, "ID: {}", self.id.unwrap())?;
        }
        writeln!(f, "Plan: {} ({})", self.plan_name, self.plan_id)?;
        writeln!(
            f,
            "Source connection: {} ({})",
            self.source_connection_name, self.source_connection_id
        )?;
        writeln!(
            f,
            "Target connection: {} ({})",
            self.target_connection_name, self.target_connection_id
        )?;

        if let Some(status) = &self.status {
            writeln!(f, "Status: {}", status)?;
        }
        if self.started_at.is_some() {
            writeln!(f, "Started at: {}", self.started_at.unwrap())?;
        }
        if self.ended_at.is_some() {
            writeln!(f, "Ended at: {}", self.ended_at.unwrap())?;
        }
        writeln!(
            f,
            "Duration: {}",
            format_duration(
                self.started_at.map(|dt| dt.and_utc()),
                self.ended_at.map(|dt| dt.and_utc())
            )
        )?;
        writeln!(f, "Total items: {}", self.items.len())?;

        if !self.items.is_empty() {
            writeln!(f, "Items:")?;
            for (i, item) in self.items.iter().enumerate() {
                writeln!(f, "{}", indent_lines(&item.to_string(), 4))?;
                if i + 1 != self.items.len() {
                    writeln!(f, "{}", indent_lines("---------------", 4))?;
                }
            }
        } else {
            writeln!(f, "No changes found")?;
        }

        let all_errors: Vec<String> = self
            .items
            .iter()
            .filter_map(|item| item.errors.clone())
            .flatten()
            .collect();

        if !all_errors.is_empty() {
            writeln!(f, "Errors:")?;
            for error in all_errors {
                writeln!(f, " - {}", error)?;
            }
        }

        Ok(())
    }
}
