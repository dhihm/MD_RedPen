//! Safe adapter for a user's authenticated Codex CLI.

use std::{
    ffi::OsStr,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

use command_group::{CommandGroup, GroupChild};
use tempfile::TempDir;

use crate::codex_error::io_error;
use crate::codex_job::cleanup_launch_failure;
pub use crate::{codex_error::CodexError, codex_job::CodexJob};

mod prompt;

pub use prompt::CodexRequest;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

/// Isolated Codex executable configuration.
pub struct CodexClient {
    executable: PathBuf,
    working_directory: PathBuf,
    _temporary_directory: Option<TempDir>,
    model: Option<String>,
    timeout: Duration,
    capture: Option<TestCapture>,
}

impl CodexClient {
    /// Creates a system Codex client in an empty temporary directory.
    ///
    /// # Errors
    ///
    /// Returns [`CodexError`] if the isolated directory cannot be created.
    pub fn system() -> Result<Self, CodexError> {
        let temporary_directory = tempfile::Builder::new()
            .prefix("md-redpen-codex-")
            .tempdir()
            .map_err(|error| io_error("create isolated directory", "md-redpen", error))?;
        let executable = std::env::var_os("MD_REDPEN_CODEX_BIN")
            .map_or_else(|| PathBuf::from("codex"), PathBuf::from);
        let model = std::env::var("MD_REDPEN_CODEX_MODEL").ok();
        Ok(Self {
            executable,
            working_directory: temporary_directory.path().to_path_buf(),
            _temporary_directory: Some(temporary_directory),
            model,
            timeout: DEFAULT_TIMEOUT,
            capture: None,
        })
    }

    /// Creates a client for a specific executable and isolated directory.
    #[must_use]
    pub fn at(executable: impl AsRef<Path>, working_directory: impl AsRef<Path>) -> Self {
        Self {
            executable: executable.as_ref().to_path_buf(),
            working_directory: working_directory.as_ref().to_path_buf(),
            _temporary_directory: None,
            model: None,
            timeout: DEFAULT_TIMEOUT,
            capture: None,
        }
    }

    /// Overrides the bounded execution time.
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Adds an explicit subscription-available model.
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Configures deterministic process-contract capture for an executable fake.
    #[doc(hidden)]
    #[must_use]
    pub fn with_test_capture(
        mut self,
        args: impl AsRef<Path>,
        stdin: impl AsRef<Path>,
        env: impl AsRef<Path>,
    ) -> Self {
        self.capture = Some(TestCapture {
            args: args.as_ref().to_path_buf(),
            stdin: stdin.as_ref().to_path_buf(),
            env: env.as_ref().to_path_buf(),
        });
        self
    }

    /// Requires an authenticated ChatGPT session without API-key overrides.
    ///
    /// # Errors
    ///
    /// Returns [`CodexError::NotChatGptLogin`] unless `codex login status`
    /// succeeds and explicitly reports a ChatGPT login.
    pub fn check_chatgpt_login(&self) -> Result<(), CodexError> {
        let output = self
            .base_command(["login", "status"])
            .output()
            .map_err(|error| io_error("run login status for", self.executable.clone(), error))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if output.status.success()
            && (stdout.contains("Logged in using ChatGPT")
                || stderr.contains("Logged in using ChatGPT"))
        {
            return Ok(());
        }
        Err(CodexError::NotChatGptLogin(
            format!("{stdout}{stderr}").trim().to_owned(),
        ))
    }

    /// Starts a cancellable noninteractive Codex process group.
    ///
    /// # Errors
    ///
    /// Returns [`CodexError`] when spawn, stdin, or pipe setup fails.
    pub fn start(&self, request: &CodexRequest) -> Result<CodexJob, CodexError> {
        let mut command = self.base_command([
            "exec",
            "--ephemeral",
            "--ignore-user-config",
            "--ignore-rules",
            "--skip-git-repo-check",
            "--sandbox",
            "read-only",
            "--color",
            "never",
        ]);
        if let Some(model) = &self.model {
            command.arg("--model").arg(model);
        }
        command
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        self.apply_capture_environment(&mut command);

        let mut child = command
            .group_spawn()
            .map_err(|error| io_error("spawn", self.executable.clone(), error))?;
        let result = self.prepare_job(&mut child, request);
        match result {
            Ok((stdout, stderr)) => Ok(CodexJob::new(child, stdout, stderr, self.timeout)),
            Err(primary) => Err(cleanup_launch_failure(child, primary)),
        }
    }

    fn base_command<I, S>(&self, args: I) -> Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = Command::new(&self.executable);
        command
            .args(args)
            .current_dir(&self.working_directory)
            .env_remove("CODEX_API_KEY")
            .env_remove("OPENAI_API_KEY");
        command
    }

    fn prepare_job(
        &self,
        child: &mut GroupChild,
        request: &CodexRequest,
    ) -> Result<
        (
            impl std::io::Read + Send + 'static,
            impl std::io::Read + Send + 'static,
        ),
        CodexError,
    > {
        let inner = child.inner();
        let mut stdin = inner.stdin.take().ok_or(CodexError::MissingPipe("stdin"))?;
        stdin
            .write_all(request.prompt().as_bytes())
            .map_err(|error| io_error("write", "codex stdin", error))?;
        drop(stdin);
        let stdout = inner
            .stdout
            .take()
            .ok_or(CodexError::MissingPipe("stdout"))?;
        let stderr = inner
            .stderr
            .take()
            .ok_or(CodexError::MissingPipe("stderr"))?;
        Ok((stdout, stderr))
    }

    fn apply_capture_environment(&self, command: &mut Command) {
        if let Some(capture) = &self.capture {
            command
                .env("FAKE_CODEX_ARGS", &capture.args)
                .env("FAKE_CODEX_STDIN", &capture.stdin)
                .env("FAKE_CODEX_ENV", &capture.env);
        }
    }
}

#[derive(Debug)]
struct TestCapture {
    args: PathBuf,
    stdin: PathBuf,
    env: PathBuf,
}
