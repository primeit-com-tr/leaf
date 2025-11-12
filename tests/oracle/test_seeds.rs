use anyhow::Result;
use serial_test::serial;

use crate::common::{
    cleanup, create_source_test_client, create_target_test_client, init_source, init_target,
    load_test_env,
};

#[tokio::test]
#[serial(oracle)]
async fn test_create_source_objects() -> Result<()> {
    load_test_env();

    let client = create_source_test_client()?;
    cleanup(&client).await?;

    init_source(&client).await?;

    let query = r#"SELECT COUNT(*) FROM dba_users WHERE username = 'SCHEMA1'"#;
    let count: i32 = client.conn.query_row(query, &[])?.get(0)?; // This line is correct as is
    assert_eq!(count, 1);

    let query = r#"
        SELECT COUNT(*) FROM dba_objects where owner = 'SCHEMA1'
        and object_name in ('EMP', 'DEPT', 'EMP_PKG')
    "#;
    let count: i32 = client.conn.query_row(query, &[])?.get(0)?; // This line is correct as is
    assert_eq!(count, 3);

    cleanup(&client).await?;

    Ok(())
}

#[tokio::test]
#[serial(oracle)]
async fn test_create_target_objects() -> Result<()> {
    load_test_env();

    let client = create_target_test_client()?;
    cleanup(&client).await?;

    init_target(&client).await?;

    let query = r#"SELECT COUNT(*) FROM dba_users WHERE username = 'SCHEMA2'"#;
    let count: i32 = client.conn.query_row(query, &[])?.get(0)?; // This line is correct as is
    assert_eq!(count, 1);

    let query = r#"
        SELECT COUNT(*) FROM dba_objects where owner = 'SCHEMA2'
        and object_name in ('SALARY', 'BONUS', 'SALARY_PKG')
    "#;
    let count: i32 = client.conn.query_row(query, &[])?.get(0)?; // This line is correct as is
    assert_eq!(count, 3);

    cleanup(&client).await?;

    Ok(())
}
