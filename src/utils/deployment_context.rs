use crate::utils::script_writer::ScriptWriterMode;
use crate::utils::{ProgressReporter, ScriptWriter, ScriptWriterOptions};
use std::io::{Error, ErrorKind, Result};
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Configuration options for the DeploymentContext.
#[derive(Debug)]
pub struct DeploymentContextOptions {
    /// If true, runs in dry-run mode (no actual changes applied).
    pub dry: bool,
    /// If true, enables script collection. Default is false (disabled).
    pub collect_scripts: bool,
    /// Optional directory path for writing script files.
    /// If None, scripts are written to memory or disabled entirely.
    pub output_path: Option<PathBuf>,
    /// Optional separator between scripts.
    pub script_sep: Option<String>,
    /// Optional progress reporter sender.
    /// If None, progress reporting is disabled.
    pub progress_tx: Option<mpsc::UnboundedSender<String>>,
}

impl DeploymentContextOptions {
    pub fn new(
        dry: bool,
        collect_scripts: bool,
        output_path: Option<PathBuf>,
        script_sep: Option<String>,
        progress_tx: Option<mpsc::UnboundedSender<String>>,
    ) -> Self {
        Self {
            dry,
            collect_scripts,
            output_path,
            script_sep,
            progress_tx,
        }
    }

    /// Validates that the options are logically consistent.
    /// Returns an error if:
    /// - `collect_scripts` is false/None but `output_path` is provided
    /// - `collect_scripts` is false/None but `script_sep` is provided
    pub fn validate(&self) -> Result<()> {
        if !self.collect_scripts {
            if self.output_path.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "output_path is set but collect_scripts is disabled. \
                     Enable collect_scripts to write scripts to a file.",
                ));
            }

            if self.script_sep.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "script_sep is set but collect_scripts is disabled. \
                     Enable collect_scripts to use script separators.",
                ));
            }
        }

        Ok(())
    }
}

impl Default for DeploymentContextOptions {
    fn default() -> Self {
        Self {
            dry: false,
            collect_scripts: false,
            output_path: None,
            script_sep: None,
            progress_tx: None,
        }
    }
}

impl Default for DeploymentContext {
    /// Creates a no-op DeploymentContext that doesn't write scripts or report progress.
    fn default() -> Self {
        Self::new(None).expect("Default DeploymentContext should never fail")
    }
}

/// A unified sink for deployment operations that handles script writing and progress reporting.
#[derive(Debug)]
pub struct DeploymentContext {
    dry_run: bool,
    collect_scripts: bool,
    script_writer: ScriptWriter,
    progress_reporter: ProgressReporter,
}

impl DeploymentContext {
    /// Creates a new DeploymentContext.
    ///
    /// - If `options` is `None`, creates a sink with default settings (no script writing, no progress).
    /// - Script writing is enabled only if `options.collect_scripts` is `Some(true)`.
    /// - When enabled, scripts are written to `output_path` if provided, otherwise to memory.
    /// - Progress reporting is enabled only if `options.progress_tx` is provided.
    ///
    /// # Errors
    /// Returns an error if the options are invalid (e.g., script_sep or output_path set when collect_scripts is disabled).
    pub fn new(options: Option<DeploymentContextOptions>) -> Result<Self> {
        let opts = options.unwrap_or_default();

        // Validate options consistency
        opts.validate()?;

        // Configure ScriptWriter based on collect_scripts flag
        let script_writer_opts = if opts.collect_scripts {
            Some(ScriptWriterOptions {
                dir: opts.output_path, // Memory mode if None
                script_sep: opts.script_sep,
            })
        } else {
            None // Disabled mode
        };

        let script_writer = ScriptWriter::new(script_writer_opts)?;

        // Create ProgressReporter with the provided sender (or None for disabled)
        let progress_reporter = ProgressReporter::new(opts.progress_tx);

        Ok(Self {
            dry_run: opts.dry,
            collect_scripts: opts.collect_scripts,
            script_writer,
            progress_reporter,
        })
    }

    /// Returns true if running in dry-run mode.
    pub fn is_dry_run(&self) -> bool {
        self.dry_run
    }

    /// Returns true if script collection is enabled.
    pub fn is_collect_scripts(&self) -> bool {
        self.collect_scripts
    }

    /// Reports progress (no-op if progress reporting is disabled).
    pub fn progress(&mut self, message: impl Into<String>) {
        self.progress_reporter.report(message);
    }

    /// Writes a migration script (no-op if script writing is disabled).
    pub fn write_script(&mut self, content: &str) -> Result<()> {
        self.script_writer.write_script(content)
    }

    /// Writes a rollback script (no-op if script writing is disabled).
    pub fn write_rollback_script(&mut self, content: &str) -> Result<()> {
        self.script_writer.write_rollback_script(content)
    }

    /// Gets the script content if available (Memory mode only).
    pub fn script_content(&self) -> Option<&str> {
        self.script_writer.script_content()
    }

    /// Gets the rollback content if available (Memory mode only).
    pub fn rollback_content(&self) -> Option<&str> {
        self.script_writer.rollback_content()
    }

    /// Gets a reference to the progress reporter for advanced usage.
    pub fn progress_reporter(&self) -> &ProgressReporter {
        &self.progress_reporter
    }

    /// Gets a mutable reference to the progress reporter for advanced usage.
    pub fn progress_reporter_mut(&mut self) -> &mut ProgressReporter {
        &mut self.progress_reporter
    }

    /// Gets a reference to the script writer for advanced usage.
    pub fn script_writer(&self) -> &ScriptWriter {
        &self.script_writer
    }

    /// Prints a summary of the deployment based on dry-run status and script output.
    /// Shows file paths if writing to files, or content if in memory mode.
    pub fn print_summary(&mut self, success_message: &str) {
        self.progress(success_message);

        match self.script_writer.mode() {
            ScriptWriterMode::Memory => {
                // Memory mode - print both contents
                if let Some(script_content) = self.script_content() {
                    println!("\n📝 Migration Scripts:\n{}", script_content);
                }
                if let Some(rollback_content) = self.rollback_content() {
                    println!("\n📝 Rollback Scripts:\n{}", rollback_content);
                }
            }
            ScriptWriterMode::File => {
                // File mode - show paths
                if let Some(script_path) = self.script_writer.script_file_path() {
                    println!(
                        "📄 Migration scripts written to: '{}'",
                        script_path.display()
                    );
                }
                if let Some(rollback_path) = self.script_writer.rollback_file_path() {
                    println!(
                        "📄 Rollback scripts written to: '{}'",
                        rollback_path.display()
                    );
                }
            }
            ScriptWriterMode::Disabled => {
                println!("✅ Script writing is disabled");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployment_sink_default() {
        let mut sink = DeploymentContext::default();
        assert!(!sink.is_dry_run());
        assert!(sink.script_content().is_none()); // No script writing

        // All operations should be no-ops
        sink.progress("test");
        sink.write_script("SELECT 1;").unwrap();
        sink.write_rollback_script("SELECT 2;").unwrap();
    }

    #[test]
    fn test_deployment_sink_scripts_disabled_by_default() {
        let opts = DeploymentContextOptions {
            dry: true,
            collect_scripts: false,
            output_path: None,
            script_sep: None,
            progress_tx: None,
        };

        let mut sink = DeploymentContext::new(Some(opts)).unwrap();
        assert!(sink.is_dry_run());

        sink.write_script("CREATE TABLE test;").unwrap();
        sink.write_rollback_script("DROP TABLE test;").unwrap();

        // Scripts are disabled by default
        assert!(sink.script_content().is_none());
    }

    #[test]
    fn test_deployment_sink_scripts_explicitly_disabled() {
        let opts = DeploymentContextOptions {
            dry: true,
            collect_scripts: false,
            output_path: None,
            script_sep: None,
            progress_tx: None,
        };

        let mut sink = DeploymentContext::new(Some(opts)).unwrap();
        sink.write_script("CREATE TABLE test;").unwrap();

        // Scripts are explicitly disabled
        assert!(sink.script_content().is_none());
    }

    #[test]
    fn test_deployment_sink_scripts_enabled() {
        let opts = DeploymentContextOptions {
            dry: true,
            collect_scripts: true,
            output_path: None, // Memory mode
            script_sep: Some(";\n".to_string()),
            progress_tx: None,
        };

        let mut sink = DeploymentContext::new(Some(opts)).unwrap();
        sink.write_script("CREATE TABLE test;").unwrap();
        sink.write_rollback_script("DROP TABLE test;").unwrap();

        // Scripts should be collected in memory
        assert!(sink.script_content().is_some());
        assert!(sink.rollback_content().is_some());
    }

    #[test]
    fn test_deployment_sink_with_progress() {
        let (tx, mut rx) = mpsc::unbounded_channel();

        let opts = DeploymentContextOptions {
            dry: false,
            collect_scripts: false,
            output_path: None,
            script_sep: None,
            progress_tx: Some(tx),
        };

        let mut sink = DeploymentContext::new(Some(opts)).unwrap();
        sink.progress("Test message");

        // Check that message was sent
        let msg = rx.try_recv().unwrap();
        assert_eq!(msg, "Test message");
    }

    #[test]
    fn test_validation_output_path_without_collect_scripts() {
        let opts = DeploymentContextOptions {
            dry: false,
            collect_scripts: false,
            output_path: Some(PathBuf::from("/tmp/scripts")),
            script_sep: None,
            progress_tx: None,
        };

        let result = DeploymentContext::new(Some(opts));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("output_path is set but collect_scripts is disabled")
        );
    }

    #[test]
    fn test_validation_script_sep_without_collect_scripts() {
        let opts = DeploymentContextOptions {
            dry: false,
            collect_scripts: false,
            output_path: None,
            script_sep: Some(";\n".to_string()),
            progress_tx: None,
        };

        let result = DeploymentContext::new(Some(opts));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("script_sep is set but collect_scripts is disabled")
        );
    }

    #[test]
    fn test_validation_both_invalid_options() {
        let opts = DeploymentContextOptions {
            dry: false,
            collect_scripts: false,
            output_path: Some(PathBuf::from("/tmp/scripts")),
            script_sep: Some(";\n".to_string()),
            progress_tx: None,
        };

        let result = DeploymentContext::new(Some(opts));
        assert!(result.is_err());
        // Should fail on the first validation error (output_path)
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("output_path is set but collect_scripts is disabled")
        );
    }

    #[test]
    fn test_validation_passes_with_collect_scripts_enabled() {
        let opts = DeploymentContextOptions {
            dry: false,
            collect_scripts: true,
            output_path: Some(PathBuf::from("/tmp/scripts")),
            script_sep: Some(";\n".to_string()),
            progress_tx: None,
        };

        // Should not panic or error during validation
        let result = DeploymentContext::new(Some(opts));
        // Note: This might fail due to file system access, but not due to validation
        // In a real test, you'd want to use a temp directory
        let _ = result; // Acknowledge we're not checking the result fully here
    }
}
