use crate::utils::script_writer::ScriptWriterMode;
use crate::utils::{ProgressReporter, ScriptWriter, ScriptWriterOptions};
use std::io::Result;
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Configuration options for the DeploymentSink.
#[derive(Debug)]
pub struct DeploymentSinkOptions {
    /// If true, runs in dry-run mode (no actual changes applied).
    pub dry: bool,
    /// Optional directory path for writing script files.
    /// If None, scripts are written to memory or disabled entirely.
    pub output_path: Option<PathBuf>,
    /// Optional separator between scripts.
    pub script_sep: Option<String>,
    /// Optional progress reporter sender.
    /// If None, progress reporting is disabled.
    pub progress_tx: Option<mpsc::UnboundedSender<String>>,
}

impl Default for DeploymentSinkOptions {
    fn default() -> Self {
        Self {
            dry: false,
            output_path: None,
            script_sep: None,
            progress_tx: None,
        }
    }
}

impl Default for DeploymentSink {
    /// Creates a no-op DeploymentSink that doesn't write scripts or report progress.
    fn default() -> Self {
        Self::new(None).expect("Default DeploymentSink should never fail")
    }
}

/// A unified sink for deployment operations that handles script writing and progress reporting.
pub struct DeploymentSink {
    dry_run: bool,
    script_writer: ScriptWriter,
    progress_reporter: ProgressReporter,
}

impl DeploymentSink {
    /// Creates a new DeploymentSink.
    ///
    /// - If `options` is `None`, creates a sink with default settings (no script writing, no progress).
    /// - Script writing is enabled only if `options.output_path` is provided.
    /// - Progress reporting is enabled only if `options.progress_tx` is provided.
    pub fn new(options: Option<DeploymentSinkOptions>) -> Result<Self> {
        let opts = options.unwrap_or_default();

        // Configure ScriptWriter based on output_path
        let script_writer_opts = match opts.output_path {
            Some(path) => Some(ScriptWriterOptions {
                dir: Some(path),
                script_sep: opts.script_sep,
            }),
            None => None, // Disabled mode
        };

        let script_writer = ScriptWriter::new(script_writer_opts)?;

        // Create ProgressReporter with the provided sender (or None for disabled)
        let progress_reporter = ProgressReporter::new(opts.progress_tx);

        Ok(Self {
            dry_run: opts.dry,
            script_writer,
            progress_reporter,
        })
    }

    /// Returns true if running in dry-run mode.
    pub fn is_dry_run(&self) -> bool {
        self.dry_run
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
        let mut sink = DeploymentSink::default();
        assert!(!sink.is_dry_run());
        assert!(sink.script_content().is_none()); // No script writing

        // All operations should be no-ops
        sink.progress("test");
        sink.write_script("SELECT 1;").unwrap();
        sink.write_rollback_script("SELECT 2;").unwrap();
    }

    #[test]
    fn test_deployment_sink_with_scripts() {
        let opts = DeploymentSinkOptions {
            dry: true,
            output_path: None,
            script_sep: Some(";\n".to_string()),
            progress_tx: None,
        };

        let mut sink = DeploymentSink::new(Some(opts)).unwrap();
        assert!(sink.is_dry_run());

        sink.write_script("CREATE TABLE test;").unwrap();
        sink.write_rollback_script("DROP TABLE test;").unwrap();

        // Scripts are disabled with None output_path
        assert!(sink.script_content().is_none());
    }

    #[test]
    fn test_deployment_sink_with_progress() {
        let (tx, mut rx) = mpsc::unbounded_channel();

        let opts = DeploymentSinkOptions {
            dry: false,
            output_path: None,
            script_sep: None,
            progress_tx: Some(tx),
        };

        let mut sink = DeploymentSink::new(Some(opts)).unwrap();
        sink.progress("Test message");

        // Check that message was sent
        let msg = rx.try_recv().unwrap();
        assert_eq!(msg, "Test message");
    }
}
