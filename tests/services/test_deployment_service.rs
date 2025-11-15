use anyhow::Result;
use leaf::{
    config::Settings,
    entities::DeploymentModel,
    oracle::OracleClient,
    services::AppServices,
    utils::{DeploymentContext, ProgressReporter},
};
use serial_test::serial;
use tempfile::NamedTempFile;

use crate::services::init_plan_test;

async fn get_target_client(
    deployment: &DeploymentModel,
    services: &AppServices,
) -> Result<OracleClient> {
    let plan = services.plan_service.get_by_id(deployment.plan_id).await?;

    let target_connection = services
        .connection_service
        .get_by_id(plan.target_connection_id)
        .await?;

    OracleClient::connect(
        &target_connection.username,
        &target_connection.password,
        &target_connection.connection_string,
    )
}

async fn check_schema1_emp_deployed(
    deployment: &DeploymentModel,
    services: &AppServices,
) -> Result<()> {
    let client = get_target_client(deployment, services).await?;
    let query = r#"SELECT column_name FROM all_tab_columns WHERE table_name = 'EMP' AND owner = 'SCHEMA1' ORDER BY column_name"#;
    let rows = client.conn.query(query, &[])?.into_iter();
    let cols: Vec<String> = rows
        .map(|row| row?.get(0))
        .collect::<Result<Vec<String>, _>>()?;

    assert_eq!(cols.len(), 1);
    assert_eq!(cols[0], "ID");
    Ok(())
}

async fn check_schema1_emp_rolled_back(
    deployment: &DeploymentModel,
    services: &AppServices,
) -> Result<()> {
    let client = get_target_client(deployment, services).await?;
    let query = r#"SELECT column_name FROM all_tab_columns WHERE table_name = 'EMP' AND owner = 'SCHEMA1' ORDER BY column_name"#;
    let rows = client.conn.query(query, &[])?.into_iter();
    let cols: Vec<String> = rows
        .map(|row| row?.get(0))
        .collect::<Result<Vec<String>, _>>()?;

    assert_eq!(cols.len(), 2);
    assert_eq!(cols[0], "ID");
    assert_eq!(cols[1], "NAME");
    Ok(())
}

async fn check_schema1_emp_columns_not_dropped(
    deployment: &DeploymentModel,
    services: &AppServices,
) -> Result<()> {
    let client = get_target_client(deployment, services).await?;
    let query = r#"SELECT column_name FROM all_tab_columns WHERE table_name = 'EMP' AND owner = 'SCHEMA1' ORDER BY column_name"#;
    let rows = client.conn.query(query, &[])?.into_iter();
    let cols: Vec<String> = rows
        .map(|row| row?.get(0))
        .collect::<Result<Vec<String>, _>>()?;

    assert_eq!(cols.len(), 2);
    assert_eq!(cols[0], "ID");
    assert_eq!(cols[1], "NAME");
    Ok(())
}

async fn check_schema1_dept(deployment: &DeploymentModel, services: &AppServices) -> Result<()> {
    let client = get_target_client(deployment, services).await?;
    let query = r#"SELECT column_name FROM all_tab_columns WHERE table_name = 'DEPT' AND owner = 'SCHEMA1'"#;
    let rows = client.conn.query(query, &[])?.into_iter();
    let cols: Vec<String> = rows
        .map(|row| row?.get(0))
        .collect::<Result<Vec<String>, _>>()?;

    assert_eq!(cols.len(), 1);
    assert_eq!(cols[0], "DEPT_ID");
    Ok(())
}

async fn check_schema2_bonus_exists(
    deployment: &DeploymentModel,
    services: &AppServices,
) -> Result<()> {
    let client = get_target_client(deployment, services).await?;
    let query =
        r#"SELECT table_name FROM all_tables WHERE table_name = 'BONUS' AND owner = 'SCHEMA2'"#;
    let rows = client.conn.query(query, &[])?.into_iter();
    let tables: Vec<String> = rows
        .map(|row| row?.get(0))
        .collect::<Result<Vec<String>, _>>()?;

    assert_eq!(tables.len(), 1);
    Ok(())
}

async fn check_schema2_bonus_not_exists(
    deployment: &DeploymentModel,
    services: &AppServices,
) -> Result<()> {
    let client = get_target_client(deployment, services).await?;
    let query =
        r#"SELECT table_name FROM all_tables WHERE table_name = 'BONUS' AND owner = 'SCHEMA2'"#;
    let rows = client.conn.query(query, &[])?.into_iter();
    let tables: Vec<String> = rows
        .map(|row| row?.get(0))
        .collect::<Result<Vec<String>, _>>()?;

    assert_eq!(tables.len(), 0);
    Ok(())
}

#[tokio::test]
#[serial(oracle)]
async fn test_run_deployment_single_schema() -> Result<()> {
    let file = NamedTempFile::new()?;
    let mut settings = Settings::new()?;
    settings.database.url = "sqlite://".to_string() + file.path().to_str().unwrap();

    init_plan_test(&settings).await?;

    let services = AppServices::new(&settings).await?;

    let plan = services
        .plan_service
        .create(
            "test",
            "source",
            "target",
            &["SCHEMA1".to_string()],
            None,
            None,
            None,
            false, // disable_all_drops
            true,  // fail_fast
            false, // disable_hooks
            None,
        )
        .await?;

    let cutoff_date = chrono::Utc::now().naive_utc() - chrono::Duration::days(1);

    let deployment_id = services
        .deployment_service
        .run(
            plan.id,
            false,
            cutoff_date,
            None,
            &mut DeploymentContext::default(),
        )
        .await?;

    match deployment_id {
        Some(deployment_id) => {
            let deployment = services.deployment_service.get_by_id(deployment_id).await?;
            check_schema1_emp_deployed(&deployment, &services).await?;
            check_schema1_dept(&deployment, &services).await?;
        }
        _ => panic!("Deployment id is not returned"),
    }

    Ok(())
}

#[tokio::test]
#[serial(oracle)]
async fn test_run_deployment() -> Result<()> {
    let file = NamedTempFile::new()?;
    let mut settings = Settings::new()?;
    settings.database.url = "sqlite://".to_string() + file.path().to_str().unwrap();

    init_plan_test(&settings).await?;

    let services = AppServices::new(&settings).await?;

    let plan = services
        .plan_service
        .create(
            "test",
            "source",
            "target",
            &["SCHEMA1".to_string(), "SCHEMA2".to_string()],
            None,
            None,
            None,
            false, // disable_all_drops
            true,  // fail_fast
            false, // disable_hooks
            None,
        )
        .await?;

    let cutoff_date = chrono::Utc::now().naive_utc() - chrono::Duration::days(1);

    let deployment = services
        .deployment_service
        .run(
            plan.id,
            false,
            cutoff_date,
            None,
            &mut DeploymentContext::default(),
        )
        .await?;

    match deployment {
        Some(deployment_id) => {
            let deployment = services.deployment_service.get_by_id(deployment_id).await?;
            check_schema1_emp_deployed(&deployment, &services).await?;
            check_schema1_dept(&deployment, &services).await?;
            check_schema2_bonus_not_exists(&deployment, &services).await?;
        }
        _ => panic!("Deployment id is not returned"),
    }

    Ok(())
}

#[tokio::test]
#[serial(oracle)]
async fn test_run_deployment_exclude_object_types() -> Result<()> {
    let file = NamedTempFile::new()?;
    let mut settings = Settings::new()?;
    settings.database.url = "sqlite://".to_string() + file.path().to_str().unwrap();

    init_plan_test(&settings).await?;

    let services = AppServices::new(&settings).await?;

    let plan = services
        .plan_service
        .create(
            "test",
            "source",
            "target",
            &["SCHEMA1".to_string(), "SCHEMA2".to_string()],
            Some(vec!["TABLE".to_string()]),
            None,
            None,
            false, // disable_all_drops
            true,  // fail_fast
            false, // disable_hooks
            None,
        )
        .await?;

    let cutoff_date = chrono::Utc::now().naive_utc() - chrono::Duration::days(1);

    let deployment = services
        .deployment_service
        .run(
            plan.id,
            false,
            cutoff_date,
            None,
            &mut DeploymentContext::default(),
        )
        .await?;

    match deployment {
        Some(deployment_id) => {
            let deployment = services.deployment_service.get_by_id(deployment_id).await?;
            // Table type is excluded, so we expect target schema2 has table named BONUS
            check_schema2_bonus_exists(&deployment, &services).await?;
        }
        _ => panic!("Deployment id is not returned"),
    }

    Ok(())
}

#[tokio::test]
#[serial(oracle)]
async fn test_run_deployment_exclude_object_names() -> Result<()> {
    let file = NamedTempFile::new()?;
    let mut settings = Settings::new()?;
    settings.database.url = "sqlite://".to_string() + file.path().to_str().unwrap();

    init_plan_test(&settings).await?;

    let services = AppServices::new(&settings).await?;

    let plan = services
        .plan_service
        .create(
            "test",
            "source",
            "target",
            &["SCHEMA1".to_string(), "SCHEMA2".to_string()],
            None,
            Some(vec!["BONUS".to_string()]),
            None,
            false, // disable_all_drops
            true,  // fail_fast
            false, // disable_hooks
            None,
        )
        .await?;

    let cutoff_date = chrono::Utc::now().naive_utc() - chrono::Duration::days(1);

    let deployment = services
        .deployment_service
        .run(
            plan.id,
            false,
            cutoff_date,
            None,
            &mut DeploymentContext::default(),
        )
        .await?;

    match deployment {
        Some(deployment_id) => {
            let deployment = services.deployment_service.get_by_id(deployment_id).await?;
            // Object name BONUS is excluded, so we expect target schema2 has table named BONUS
            check_schema2_bonus_exists(&deployment, &services).await?;
        }
        _ => panic!("Deployment id is not returned"),
    }

    Ok(())
}

#[tokio::test]
#[serial(oracle)]
async fn test_run_deployment_disabled_drop_types() -> Result<()> {
    let file = NamedTempFile::new()?;
    let mut settings = Settings::new()?;
    settings.database.url = "sqlite://".to_string() + file.path().to_str().unwrap();

    init_plan_test(&settings).await?;

    let services = AppServices::new(&settings).await?;

    let plan = services
        .plan_service
        .create(
            "test",
            "source",
            "target",
            &["SCHEMA1".to_string(), "SCHEMA2".to_string()],
            None,
            None,
            Some(vec!["TABLE".to_string(), "COLUMN".to_string()]),
            false, // disable_all_drops
            true,  // fail_fast
            false, // disable_hooks
            None,
        )
        .await?;

    let cutoff_date = chrono::Utc::now().naive_utc() - chrono::Duration::days(1);

    let deployment = services
        .deployment_service
        .run(
            plan.id,
            false,
            cutoff_date,
            None,
            &mut DeploymentContext::default(),
        )
        .await?;

    match deployment {
        Some(deployment_id) => {
            let deployment = services.deployment_service.get_by_id(deployment_id).await?;
            // drop TABLE is disabled, so we expect target schema2 has table named BONUS
            check_schema2_bonus_exists(&deployment, &services).await?;
            // drop COLUMN is disabled, so we expect SCHEMA1.EMP still has NAME column
            check_schema1_emp_columns_not_dropped(&deployment, &services).await?;
        }
        _ => panic!("Deployment id is not returned"),
    }

    Ok(())
}

#[tokio::test]
#[serial(oracle)]
async fn test_run_deployment_with_disable_all_drops() -> Result<()> {
    let file = NamedTempFile::new()?;
    let mut settings = Settings::new()?;
    settings.database.url = "sqlite://".to_string() + file.path().to_str().unwrap();

    init_plan_test(&settings).await?;

    let services = AppServices::new(&settings).await?;

    let plan = services
        .plan_service
        .create(
            "test",
            "source",
            "target",
            &["SCHEMA1".to_string(), "SCHEMA2".to_string()],
            None,
            None,
            None,
            true,  // disable_all_drops
            true,  // fail_fast
            false, // disable_hooks
            None,
        )
        .await?;

    let cutoff_date = chrono::Utc::now().naive_utc() - chrono::Duration::days(1);

    let deployment = services
        .deployment_service
        .run(
            plan.id,
            false,
            cutoff_date,
            None,
            &mut DeploymentContext::default(),
        )
        .await?;

    match deployment {
        Some(deployment_id) => {
            let deployment = services.deployment_service.get_by_id(deployment_id).await?;
            // disable_all_drops means no table drops, so BONUS should still exist
            check_schema2_bonus_exists(&deployment, &services).await?;
            // disable_all_drops also filters DROP COLUMN, so NAME should still exist
            check_schema1_emp_columns_not_dropped(&deployment, &services).await?;
        }
        _ => panic!("Deployment id is not returned"),
    }

    Ok(())
}

#[tokio::test]
#[serial(oracle)]
async fn test_run_deployment_disable_all_drops_vs_disabled_drop_types() -> Result<()> {
    let file = NamedTempFile::new()?;
    let mut settings = Settings::new()?;
    settings.database.url = "sqlite://".to_string() + file.path().to_str().unwrap();

    init_plan_test(&settings).await?;

    let services = AppServices::new(&settings).await?;

    // Test 1: disable_all_drops = true should behave the same as disabled_drop_types = [COLUMN]
    // but also skip target object processing (no DROP TABLE operations)
    let plan1 = services
        .plan_service
        .create(
            "test_disable_all",
            "source",
            "target",
            &["SCHEMA1".to_string(), "SCHEMA2".to_string()],
            None,
            None,
            None,
            true,  // disable_all_drops
            true,  // fail_fast
            false, // disable_hooks
            None,
        )
        .await?;

    let cutoff_date = chrono::Utc::now().naive_utc() - chrono::Duration::days(1);

    let deployment1 = services
        .deployment_service
        .run(
            plan1.id,
            false,
            cutoff_date,
            None,
            &mut DeploymentContext::default(),
        )
        .await?;

    if let Some(deployment_id) = deployment1 {
        let deployment = services.deployment_service.get_by_id(deployment_id).await?;
        // Both TABLE and COLUMN drops should be disabled
        check_schema2_bonus_exists(&deployment, &services).await?;
        check_schema1_emp_columns_not_dropped(&deployment, &services).await?;
    } else {
        panic!("Deployment id is not returned");
    }

    Ok(())
}

#[tokio::test]
#[serial(oracle)]
async fn test_rollback_deployment() -> Result<()> {
    let file = NamedTempFile::new()?;
    let mut settings = Settings::new()?;
    settings.database.url = "sqlite://".to_string() + file.path().to_str().unwrap();

    init_plan_test(&settings).await?;

    let services = AppServices::new(&settings).await?;

    let plan = services
        .plan_service
        .create(
            "test",
            "source",
            "target",
            &["SCHEMA1".to_string(), "SCHEMA2".to_string()],
            None,
            None,
            None,
            false, // disable_all_drops
            true,  // fail_fast
            false, // disable_hooks
            None,
        )
        .await?;

    let plan_id = plan.id.clone();
    let cutoff_date = chrono::Utc::now().naive_utc() - chrono::Duration::days(1);

    let result = services
        .deployment_service
        .run(
            plan_id,
            false,
            cutoff_date,
            None,
            &mut DeploymentContext::default(),
        )
        .await?;

    match result {
        Some(deployment_id) => {
            let deployment = services.deployment_service.get_by_id(deployment_id).await?;
            check_schema1_emp_deployed(&deployment, &services).await?;
        }
        _ => panic!("Deployment id is not returned"),
    }

    services
        .deployment_service
        .rollback(plan_id, ProgressReporter::new(None))
        .await?;

    check_schema1_emp_rolled_back(&services.deployment_service.get_by_id(1).await?, &services)
        .await?;

    Ok(())
}
