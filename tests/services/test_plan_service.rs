use anyhow::Result;
use leaf::{
    config::Settings,
    services::AppServices,
    types::{PlanStatus, StringList},
};
use tempfile::NamedTempFile;

use crate::common::{
    cleanup, create_source_objects, create_source_test_client, create_target_objects,
    create_target_test_client, init_repo, load_test_env,
};

pub async fn create_connections(settings: &Settings) -> Result<()> {
    load_test_env();

    let services = AppServices::new(settings).await?;
    init_repo(&services.connection_service.get_repo().get_db().await).await?;

    let username = std::env::var("TEST_SOURCE_ORACLE_USER").unwrap_or("system".to_string());
    let password = std::env::var("TEST_SOURCE_ORACLE_PASSWORD").unwrap_or("oracle".to_string());
    let connection_string = std::env::var("TEST_SOURCE_ORACLE_CONNECTION_STRING")
        .unwrap_or("localhost:1521/XEPDB1".to_string());

    services
        .connection_service
        .create(
            "source",
            username.as_str(),
            password.as_str(),
            connection_string.as_str(),
        )
        .await?;

    let username = std::env::var("TEST_TARGET_ORACLE_USER").unwrap_or("system".to_string());
    let password = std::env::var("TEST_TARGET_ORACLE_PASSWORD").unwrap_or("oracle".to_string());
    let connection_string = std::env::var("TEST_TARGET_ORACLE_CONNECTION_STRING")
        .unwrap_or("localhost:1522/XEPDB1".to_string());

    services
        .connection_service
        .create(
            "target",
            username.as_str(),
            password.as_str(),
            connection_string.as_str(),
        )
        .await?;

    Ok(())
}

pub async fn seed_db_objects() -> Result<()> {
    let source_client = create_source_test_client()?;
    let target_client = create_target_test_client()?;

    cleanup(&source_client).await?;
    cleanup(&target_client).await?;
    create_source_objects(&source_client).await?;
    create_target_objects(&target_client).await?;
    Ok(())
}

pub async fn init_plan_test(settings: &Settings) -> Result<()> {
    create_connections(settings).await?;
    seed_db_objects().await
}

#[tokio::test]
async fn test_create_plan() -> Result<()> {
    let file = NamedTempFile::new()?;
    let mut settings = Settings::new()?;
    settings.database.url = "sqlite://".to_string() + file.path().to_str().unwrap();

    create_connections(&settings).await?;

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
            false,
            true,
            None,
        )
        .await?;

    assert_eq!(plan.name, "test");
    assert_eq!(plan.source_connection_id > 0, true);
    assert_eq!(plan.target_connection_id > 0, true);
    assert_eq!(plan.source_connection_id != plan.target_connection_id, true);
    assert_eq!(plan.schemas.0.len(), 1);
    assert_eq!(plan.exclude_object_types.is_some(), true);
    println!("{:?}", plan.exclude_object_names);
    assert_eq!(plan.exclude_object_names, Some(StringList(vec![])));

    let res = services
        .plan_service
        .create(
            "TEST",
            "source",
            "target",
            &["SCHEMA1".to_string()],
            None,
            None,
            None,
            false,
            true,
            None,
        )
        .await;
    assert!(res.is_err());

    let res = services
        .plan_service
        .create(
            "valid",
            "source",
            "target",
            &["SCHEMA1".to_string()],
            None,
            None,
            None,
            false,
            true,
            None,
        )
        .await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap().name, "valid");

    Ok(())
}

#[tokio::test]
async fn test_find_by_name() -> Result<()> {
    let file = NamedTempFile::new()?;
    let mut settings = Settings::new()?;
    settings.database.url = "sqlite://".to_string() + file.path().to_str().unwrap();

    create_connections(&settings).await?;

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
            false,
            true,
            None,
        )
        .await?;

    assert_eq!(plan.name, "test");
    assert_eq!(plan.source_connection_id > 0, true);
    assert_eq!(plan.target_connection_id > 0, true);
    assert_eq!(plan.source_connection_id != plan.target_connection_id, true);
    assert_eq!(plan.schemas.0.len(), 1);
    assert_eq!(plan.exclude_object_types.is_some(), true);
    assert_eq!(plan.exclude_object_names, Some(StringList(vec![])));

    let res = services.plan_service.find_by_name("test").await?;
    assert_eq!(res.unwrap().name, "test");

    Ok(())
}

#[tokio::test]
async fn test_get_by_id() -> Result<()> {
    let file = NamedTempFile::new()?;
    let mut settings = Settings::new()?;
    settings.database.url = "sqlite://".to_string() + file.path().to_str().unwrap();

    create_connections(&settings).await?;

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
            false,
            true,
            None,
        )
        .await?;

    assert_eq!(plan.name, "test");
    assert_eq!(plan.source_connection_id > 0, true);
    assert_eq!(plan.target_connection_id > 0, true);
    assert_eq!(plan.source_connection_id != plan.target_connection_id, true);
    assert_eq!(plan.schemas.0.len(), 1);
    assert_eq!(plan.exclude_object_types.is_some(), true);
    assert_eq!(plan.exclude_object_names, Some(StringList(vec![])));

    let res = services.plan_service.get_by_id(plan.id).await?;
    assert_eq!(res.name, "test");

    Ok(())
}

#[tokio::test]
async fn test_get_all() -> Result<()> {
    let file = NamedTempFile::new()?;
    let mut settings = Settings::new()?;
    settings.database.url = "sqlite://".to_string() + file.path().to_str().unwrap();

    create_connections(&settings).await?;

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
            false,
            true,
            None,
        )
        .await?;

    assert_eq!(plan.name, "test");
    assert_eq!(plan.source_connection_id > 0, true);
    assert_eq!(plan.target_connection_id > 0, true);
    assert_eq!(plan.source_connection_id != plan.target_connection_id, true);
    assert_eq!(plan.schemas.0.len(), 1);
    assert_eq!(plan.exclude_object_types.is_some(), true);
    assert_eq!(plan.exclude_object_names, Some(StringList(vec![])));

    let res = services.plan_service.get_all().await?;
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].name, "test");

    Ok(())
}

#[tokio::test]
async fn test_get_by_status() -> Result<()> {
    let file = NamedTempFile::new()?;
    let mut settings = Settings::new()?;
    settings.database.url = "sqlite://".to_string() + file.path().to_str().unwrap();

    create_connections(&settings).await?;

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
            false,
            true,
            None,
        )
        .await?;

    assert_eq!(plan.name, "test");
    assert_eq!(plan.source_connection_id > 0, true);
    assert_eq!(plan.target_connection_id > 0, true);
    assert_eq!(plan.source_connection_id != plan.target_connection_id, true);
    assert_eq!(plan.schemas.0.len(), 1);
    assert_eq!(plan.exclude_object_types.is_some(), true);
    assert_eq!(plan.exclude_object_names, Some(StringList(vec![])));

    let res = services
        .plan_service
        .get_by_status(PlanStatus::Running)
        .await?;
    assert_eq!(res.len(), 0);

    let res = services
        .plan_service
        .get_by_status(PlanStatus::Success)
        .await?;
    assert_eq!(res.len(), 0);

    let res = services
        .plan_service
        .get_by_status(PlanStatus::Error)
        .await?;
    assert_eq!(res.len(), 0);

    let res = services
        .plan_service
        .get_by_status(PlanStatus::Idle)
        .await?;
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].name, "test");

    Ok(())
}

#[tokio::test]
async fn test_prune() -> Result<()> {
    let file = NamedTempFile::new()?;
    let mut settings = Settings::new()?;
    settings.database.url = "sqlite://".to_string() + file.path().to_str().unwrap();

    create_connections(&settings).await?;

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
            false,
            true,
            None,
        )
        .await?;
    assert_eq!(plan.name, "test");
    assert_eq!(plan.source_connection_id > 0, true);
    assert_eq!(plan.target_connection_id > 0, true);
    assert_eq!(plan.source_connection_id != plan.target_connection_id, true);
    assert_eq!(plan.schemas.0.len(), 1);
    assert_eq!(plan.exclude_object_types.is_some(), true);
    assert_eq!(plan.exclude_object_names, Some(StringList(vec![])));

    let res = services.plan_service.prune().await?;
    assert_eq!(res, 1);

    let res = services.plan_service.get_all().await?;
    assert_eq!(res.len(), 0);

    Ok(())
}
