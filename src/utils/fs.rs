use std::path::PathBuf;

pub fn validate_dir(path: &str) -> Result<PathBuf, String> {
    let pb = PathBuf::from(path);
    if pb.is_dir() {
        Ok(pb)
    } else {
        Err(format!("{} is not a valid directory", path))
    }
}
