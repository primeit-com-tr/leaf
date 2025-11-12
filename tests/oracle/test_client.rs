use leaf::oracle::OracleClient;

use crate::common::load_test_env;

#[test]
pub fn test_oracle_client() {
    load_test_env();
    let username = std::env::var("TEST_SOURCE_ORACLE_USER").unwrap_or("system".to_string());
    let password = std::env::var("TEST_SOURCE_ORACLE_PASSWORD").unwrap_or("oracle".to_string());
    let connection_string = std::env::var("TEST_SOURCE_ORACLE_CONNECTION_STRING")
        .unwrap_or("localhost:1521/XEPDB1".to_string());
    let client = OracleClient::connect(
        username.as_str(),
        password.as_str(),
        connection_string.as_str(),
    );
    assert!(client.is_ok());
}

#[test]
pub fn test_invalid_connection() {
    load_test_env();
    let username = "invalid_user";
    let password = std::env::var("TEST_SOURCE_ORACLE_PASSWORD").unwrap_or("oracle".to_string());
    let connection_string = std::env::var("TEST_SOURCE_ORACLE_CONNECTION_STRING")
        .unwrap_or("localhost:1521/INVALID".to_string());
    let client = OracleClient::connect(username, password.as_str(), connection_string.as_str());
    assert!(client.is_err());
}
