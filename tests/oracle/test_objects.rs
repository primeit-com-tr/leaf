use anyhow::Result;

use crate::common::{create_source_test_client, load_test_env};

#[tokio::test]
async fn test_get_objects_by_status() -> Result<()> {
    load_test_env();
    let client = create_source_test_client()?;
    let objects = client.get_objects_by_status(Some("INVALID")).await;
    assert!(
        objects.is_ok(),
        "Failed to get objects by status: {:?}",
        objects
    );
    Ok(())
}
