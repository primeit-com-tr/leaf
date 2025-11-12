use thiserror::Error;

#[derive(Debug, thiserror::Error)]
pub enum SchemaValidationError {
    #[error("The following schemas were not found: {schemas}")]
    MissingSchemas { schemas: String },
}

impl SchemaValidationError {
    pub fn from_vec(missing: Vec<String>) -> Self {
        Self::MissingSchemas {
            schemas: missing.join(", "),
        }
    }
}

#[derive(Error, Debug)]
pub enum DeployError {
    #[error("Deployment failed with {0} errors: {1:?}")]
    Errors(usize, Vec<String>),
}
