use dotenvy::from_filename;

pub fn load_test_env() {
    // Explicitly load environment variables from .env.test
    from_filename(".env.test").ok();
}
