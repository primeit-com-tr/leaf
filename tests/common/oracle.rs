use anyhow::Result;
use leaf::oracle::OracleClient;

pub fn create_source_test_client() -> Result<OracleClient> {
    let username = std::env::var("TEST_SOURCE_ORACLE_USER").unwrap_or("system".to_string());
    let password = std::env::var("TEST_SOURCE_ORACLE_PASSWORD").unwrap_or("oracle".to_string());
    let connection_string = std::env::var("TEST_SOURCE_ORACLE_CONNECTION_STRING")
        .unwrap_or("localhost:1521/XEPDB1".to_string());
    OracleClient::connect(&username, &password, &connection_string)
}

pub fn create_target_test_client() -> Result<OracleClient> {
    let username = std::env::var("TEST_TARGET_ORACLE_USER").unwrap_or("system".to_string());
    let password = std::env::var("TEST_TARGET_ORACLE_PASSWORD").unwrap_or("oracle".to_string());
    let connection_string = std::env::var("TEST_TARGET_ORACLE_CONNECTION_STRING")
        .unwrap_or("localhost:1522/XEPDB1".to_string());
    OracleClient::connect(&username, &password, &connection_string)
}
