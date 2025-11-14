use chrono::Local;
use std::fs::OpenOptions;
use std::io::{Result, Write};
use std::path::{Path, PathBuf};

use crate::utils::normalize_sql;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptWriterMode {
    /// Scripts are disabled (no writing).
    Disabled,
    /// Scripts are written to files.
    File,
    /// Scripts are written to memory buffers.
    Memory,
}

/// The target destination for script content.
#[derive(Debug)]
pub enum ScriptTarget {
    /// Writes content to a physical file path.
    File(PathBuf),
    /// Writes content to an in-memory buffer.
    Memory(String),
}

/// Configuration options for the ScriptWriter.
#[derive(Debug)]
pub struct ScriptWriterOptions {
    /// The directory path where script files should be written.
    /// If this is `Some`, the writer operates in File mode. If `None`, it operates in Memory mode.
    pub dir: Option<PathBuf>,
    /// The separator string to be placed between scripts.
    /// If `None`, the default separator ("\n\n") is used.
    pub script_sep: Option<String>,
}

/// A writer utility for creating migration and rollback scripts.
/// It supports writing to timestamped files or in-memory strings.
#[derive(Debug)]
pub struct ScriptWriter {
    _dir: Option<PathBuf>,
    // These targets are now wrapped in Option. If None, writing is disabled.
    script_target: Option<ScriptTarget>,
    rollback_target: Option<ScriptTarget>,
    // Separator is also optional, only present if writing is enabled.
    script_sep: Option<String>,

    mode: ScriptWriterMode,
}

impl ScriptWriter {
    /// Creates a new ScriptWriter.
    ///
    /// - If `options` is `None`, the writer is initialized in a disabled state,
    ///   and all calls to `write_script` will be no-ops.
    /// - If `options.dir` is `Some`, it writes to files (File mode).
    /// - If `options.dir` is `None`, it writes to in-memory buffers (Memory mode).
    pub fn new(options: Option<ScriptWriterOptions>) -> Result<Self> {
        let opts = match options {
            Some(o) => o,
            None => {
                return Ok(Self {
                    _dir: None,
                    script_target: None,
                    rollback_target: None,
                    script_sep: None,
                    mode: ScriptWriterMode::Disabled,
                });
            }
        };

        let script_sep = opts.script_sep.unwrap_or_else(|| "\n\n".to_string());

        match opts.dir {
            Some(dir) => {
                // File mode
                std::fs::create_dir_all(&dir)?;
                let timestamp = Local::now().format("%Y%m%d-%H%M%S").to_string();

                let script_file = dir.join(format!("scripts-{}.sql", timestamp));
                let rollback_file = dir.join(format!("rollback_scripts-{}.sql", timestamp));

                std::fs::File::create(&script_file)?;
                std::fs::File::create(&rollback_file)?;

                Ok(Self {
                    _dir: Some(dir),
                    script_target: Some(ScriptTarget::File(script_file)),
                    rollback_target: Some(ScriptTarget::File(rollback_file)),
                    script_sep: Some(script_sep),
                    mode: ScriptWriterMode::File, // Add this
                })
            }
            None => Ok(Self {
                // Memory mode
                _dir: None,
                script_target: Some(ScriptTarget::Memory(String::new())),
                rollback_target: Some(ScriptTarget::Memory(String::new())),
                script_sep: Some(script_sep),
                mode: ScriptWriterMode::Memory, // Add this
            }),
        }
    }

    /// Returns the current mode of the script writer.
    pub fn mode(&self) -> ScriptWriterMode {
        self.mode
    }

    /// Appends content followed by the defined separator to the main script target.
    /// If the writer is disabled, this is a no-op.
    pub fn write_script(&mut self, content: &str) -> Result<()> {
        let target = match self.script_target.as_mut() {
            Some(t) => t,
            None => return Ok(()),
        };

        let sep = self.script_sep.as_ref().unwrap();
        let normalized = normalize_sql(content);

        Self::append(target, &normalized)?;
        Self::append(target, sep)
    }

    /// Appends content followed by the defined separator to the rollback script target.
    /// If the writer is disabled, this is a no-op.
    pub fn write_rollback_script(&mut self, content: &str) -> Result<()> {
        let target = match self.rollback_target.as_mut() {
            Some(t) => t,
            None => return Ok(()),
        };

        let sep = self.script_sep.as_ref().unwrap();
        let normalized = normalize_sql(content);

        Self::append(target, &normalized)?;
        Self::append(target, sep)
    }

    /// Internal static function to append content to a target.
    /// It avoids adding an implicit newline, ensuring cleaner separation control.
    fn append(target: &mut ScriptTarget, content: &str) -> Result<()> {
        match target {
            ScriptTarget::File(path) => {
                // Use write_all instead of writeln! to avoid adding an automatic newline.
                let mut file = OpenOptions::new().create(true).append(true).open(path)?;
                file.write_all(content.as_bytes())?;
                Ok(())
            }
            ScriptTarget::Memory(buf) => {
                buf.push_str(content);
                Ok(())
            }
        }
    }

    /// Retrieves content if the writer is in Memory mode and is enabled.
    pub fn script_content(&self) -> Option<&str> {
        match self.script_target.as_ref() {
            Some(ScriptTarget::Memory(s)) => Some(s),
            _ => None,
        }
    }

    /// Retrieves rollback content if the writer is in Memory mode and is enabled.
    pub fn rollback_content(&self) -> Option<&str> {
        match self.rollback_target.as_ref() {
            Some(ScriptTarget::Memory(s)) => Some(s),
            _ => None,
        }
    }

    pub fn get_script_target(&self) -> Option<&ScriptTarget> {
        self.script_target.as_ref()
    }

    /// Returns the path to the script file if writing to a file.
    pub fn script_file_path(&self) -> Option<&Path> {
        match &self.script_target {
            Some(ScriptTarget::File(path)) => Some(path.as_path()),
            _ => None,
        }
    }

    /// Returns the path to the rollback script file if writing to a file.
    pub fn rollback_file_path(&self) -> Option<&Path> {
        match &self.rollback_target {
            Some(ScriptTarget::File(path)) => Some(path.as_path()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // Helper to get file content (and normalize line endings for robust testing)
    fn read_and_normalize(path: &Path) -> Result<String> {
        let content = fs::read_to_string(path)?;
        Ok(content.replace("\r\n", "\n"))
    }

    #[test]
    fn test_skip_all_writes_when_options_are_none() -> Result<()> {
        // Initialize in disabled mode (no options)
        let mut writer = ScriptWriter::new(None)?;

        writer.write_script("SHOULD NOT BE WRITTEN")?;
        writer.write_rollback_script("SHOULD NOT BE WRITTEN")?;

        // In disabled mode, targets are None, so content access should also be None.
        assert!(writer.script_content().is_none());
        assert!(writer.rollback_content().is_none());

        Ok(())
    }

    #[test]
    fn test_file_mode_files_created() -> Result<()> {
        let tmp = tempdir()?;
        let options = Some(ScriptWriterOptions {
            dir: Some(tmp.path().to_path_buf()),
            script_sep: None,
        });
        let writer = ScriptWriter::new(options)?;

        if let Some(ScriptTarget::File(ref path)) = writer.script_target {
            assert!(path.exists());
        } else {
            // This panic now requires the Debug implementation on ScriptTarget
            panic!("Expected file target, got {:?}", writer.script_target);
        }

        Ok(())
    }

    #[test]
    fn test_write_to_files_clean_separation() -> Result<()> {
        let tmp = tempdir()?;
        let options = Some(ScriptWriterOptions {
            dir: Some(tmp.path().to_path_buf()),
            script_sep: Some("--SEP--\n".to_string()),
        });
        let mut writer = ScriptWriter::new(options)?;

        writer.write_script("CREATE TABLE users (id INT);")?;
        writer.write_script("INSERT INTO users VALUES (1);")?;

        if let Some(ScriptTarget::File(ref path)) = writer.script_target {
            let content = read_and_normalize(path)?;

            // Expect content followed by separator, twice.
            let expected_content =
                "CREATE TABLE users (id INT);--SEP--\nINSERT INTO users VALUES (1);--SEP--\n";
            assert_eq!(content, expected_content);
        }

        Ok(())
    }

    #[test]
    fn test_memory_mode_writes_to_buffers_clean_separation() -> Result<()> {
        let options = Some(ScriptWriterOptions {
            dir: None, // Explicitly set dir to None for memory mode
            script_sep: Some(";\n".to_string()),
        });
        let mut writer = ScriptWriter::new(options)?;

        writer.write_script("INSERT INTO x VALUES (1)")?;
        writer.write_rollback_script("DELETE FROM x")?;
        writer.write_script("INSERT INTO y VALUES (2)")?;

        let script = writer.script_content().unwrap();
        let rollback = writer.rollback_content().unwrap();

        // Check script content with explicit separator (';\n')
        assert_eq!(
            script,
            "INSERT INTO x VALUES (1);\nINSERT INTO y VALUES (2);\n"
        );
        assert_eq!(rollback, "DELETE FROM x;\n");

        Ok(())
    }

    #[test]
    fn test_default_separator_in_memory() -> Result<()> {
        let options = Some(ScriptWriterOptions {
            dir: None,        // Memory mode
            script_sep: None, // Use default separator ("\n\n")
        });
        let mut writer = ScriptWriter::new(options)?;

        writer.write_script("Statement 1")?;
        writer.write_script("Statement 2")?;

        let script = writer.script_content().unwrap();

        // Default separator is "\n\n"
        let expected_content = "Statement 1\n\nStatement 2\n\n";
        assert_eq!(script, expected_content);

        Ok(())
    }

    #[test]
    fn test_default_separator_in_files() -> Result<()> {
        let tmp = tempdir()?;
        let options = Some(ScriptWriterOptions {
            dir: Some(tmp.path().to_path_buf()),
            script_sep: None, // Use default separator ("\n\n")
        });
        let mut writer = ScriptWriter::new(options)?;

        writer.write_script("Statement A;")?;
        writer.write_script("Statement B;")?;

        if let Some(ScriptTarget::File(ref path)) = writer.script_target {
            let content = read_and_normalize(path)?;

            // Default separator is "\n\n"
            let expected_content = "Statement A;\n\nStatement B;\n\n";
            assert_eq!(content, expected_content);
        }

        Ok(())
    }
}
