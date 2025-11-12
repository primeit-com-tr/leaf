use anyhow::Result;
use leaf::{config::Settings, entities::ConnectionModel, services::AppServices};

use crate::common::{init_repo, load_test_env};

#[tokio::test]
async fn test_create_connection() -> Result<()> {
    load_test_env();
    let settings = Settings::new()?;
    let services = AppServices::new(&settings).await?;
    init_repo(&services.connection_service.get_repo().get_db().await).await?;

    fn assert_connection(connection: &ConnectionModel) {
        assert_eq!(connection.name, "test");
        assert_eq!(connection.username, "test");
        assert_eq!(connection.password, "test");
        assert_eq!(connection.connection_string, "test");
    }

    let connection = services
        .connection_service
        .create("test", "test", "test", "test")
        .await?;

    assert_connection(&connection);
    let connection = services.connection_service.get_by_id(connection.id).await?;
    assert_connection(&connection);

    Ok(())
}

#[tokio::test]
async fn test_create_connection_with_duplicate_name() -> Result<()> {
    load_test_env();
    let settings = Settings::new()?;
    let services = AppServices::new(&settings).await?;
    init_repo(&services.connection_service.get_repo().get_db().await).await?;

    let res = services
        .connection_service
        .create("test", "test", "test", "test")
        .await;
    assert!(res.is_ok());

    let res = services
        .connection_service
        .create("TeST", "test", "test", "test")
        .await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn test_find_connection_by_name() -> Result<()> {
    load_test_env();
    let settings = Settings::new()?;
    let services = AppServices::new(&settings).await?;
    init_repo(&services.connection_service.get_repo().get_db().await).await?;

    fn assert_connection(connection: &ConnectionModel) {
        assert_eq!(connection.name, "test");
        assert_eq!(connection.username, "test");
        assert_eq!(connection.password, "test");
        assert_eq!(connection.connection_string, "test");
    }

    let connection = services
        .connection_service
        .create("test", "test", "test", "test")
        .await?;

    assert_connection(&connection);
    if let Some(connection) = services.connection_service.find_by_name("test").await? {
        assert_connection(&connection);
    } else {
        panic!("Connection not found");
    }
    if let Some(_) = services.connection_service.find_by_name("invalid").await? {
        panic!("Connection found for invalid name");
    }
    Ok(())
}

#[tokio::test]
async fn test_delete_connection_by_name() -> Result<()> {
    load_test_env();
    let settings = Settings::new()?;
    let services = AppServices::new(&settings).await?;
    init_repo(&services.connection_service.get_repo().get_db().await).await?;

    let connection = services
        .connection_service
        .create("test", "test", "test", "test")
        .await?;

    let connection_name = connection.name.clone();

    assert!(
        services
            .connection_service
            .delete_by_name(&connection_name)
            .await
            .is_ok()
    );
    if let Some(_) = services.connection_service.find_by_name("test").await? {
        panic!("Connection found after deletion");
    }
    Ok(())
}

#[tokio::test]
async fn test_prune_connections() -> Result<()> {
    load_test_env();
    let settings = Settings::new()?;
    let services = AppServices::new(&settings).await?;
    init_repo(&services.connection_service.get_repo().get_db().await).await?;

    services
        .connection_service
        .create("test", "test", "test", "test")
        .await?;

    services
        .connection_service
        .create("test2", "test", "test", "test")
        .await?;

    services
        .connection_service
        .create("test3", "test", "test", "test")
        .await?;

    assert!(services.connection_service.prune().await.is_ok());
    if let Some(_) = services.connection_service.find_by_name("test").await? {
        panic!("Connection found after pruning");
    }
    if let Some(_) = services.connection_service.find_by_name("test2").await? {
        panic!("Connection found after pruning");
    }
    if let Some(_) = services.connection_service.find_by_name("test3").await? {
        panic!("Connection found after pruning");
    }
    Ok(())
}

#[tokio::test]
async fn test_get_all_connections() -> Result<()> {
    load_test_env();
    let settings = Settings::new()?;
    let services = AppServices::new(&settings).await?;
    init_repo(&services.connection_service.get_repo().get_db().await).await?;

    services
        .connection_service
        .create("test", "test", "test", "test")
        .await?;

    services
        .connection_service
        .create("test2", "test", "test", "test")
        .await?;

    services
        .connection_service
        .create("test3", "test", "test", "test")
        .await?;
    let connections = services.connection_service.get_all().await?;
    assert_eq!(connections.len(), 3);
    Ok(())
}

#[tokio::test]
async fn test_ping_connection() -> Result<()> {
    load_test_env();
    let settings = Settings::new()?;
    let services = AppServices::new(&settings).await?;
    init_repo(&services.connection_service.get_repo().get_db().await).await?;

    let username = std::env::var("TEST_SOURCE_ORACLE_USER").unwrap_or("system".to_string());
    let password = std::env::var("TEST_SOURCE_ORACLE_PASSWORD").unwrap_or("oracle".to_string());
    let connection_string = std::env::var("TEST_SOURCE_ORACLE_CONNECTION_STRING")
        .unwrap_or("localhost:1521/XEPDB1".to_string());
    services
        .connection_service
        .create(
            "test",
            username.as_str(),
            password.as_str(),
            connection_string.as_str(),
        )
        .await?;

    let connection = services.connection_service.get_by_id(1).await?;
    assert_eq!(connection.name, "test");
    let res = services.connection_service.ping("test").await?;
    assert_eq!(res.name, "test");

    services
        .connection_service
        .create(
            "test2",
            username.as_str(),
            "invalid_password",
            connection_string.as_str(),
        )
        .await?;

    let connection = services.connection_service.get_by_id(2).await?;
    assert_eq!(connection.name, "test2");
    let res = services.connection_service.ping("test2").await;
    assert!(res.is_err());

    let res = services.connection_service.ping("invalid").await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn test_test_connection() -> Result<()> {
    load_test_env();
    let settings = Settings::new()?;
    let services = AppServices::new(&settings).await?;
    init_repo(&services.connection_service.get_repo().get_db().await).await?;

    let username = std::env::var("TEST_SOURCE_ORACLE_USER").unwrap_or("system".to_string());
    let password = std::env::var("TEST_SOURCE_ORACLE_PASSWORD").unwrap_or("oracle".to_string());
    let connection_string = std::env::var("TEST_SOURCE_ORACLE_CONNECTION_STRING")
        .unwrap_or("localhost:1521/XEPDB1".to_string());

    let res = services
        .connection_service
        .test(
            username.as_str(),
            password.as_str(),
            connection_string.as_str(),
        )
        .await;

    assert!(res.is_ok());

    let res = services
        .connection_service
        .test(
            username.as_str(),
            "invalid_password",
            connection_string.as_str(),
        )
        .await;

    assert!(res.is_err());

    Ok(())
}
