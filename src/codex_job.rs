//! Running Codex process-group lifecycle.

use std::{
    io::Read,
    process::ExitStatus,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use command_group::GroupChild;
use wait_timeout::ChildExt;

use crate::codex_error::{CodexError, io_error};

const MAX_OUTPUT_BYTES: usize = 64 * 1024;

/// One cancellable, bounded Codex subprocess.
pub struct CodexJob {
    child: Option<GroupChild>,
    stdout: Option<JoinHandle<Result<String, CodexError>>>,
    stderr: Option<JoinHandle<Result<String, CodexError>>>,
    started: Instant,
    timeout: Duration,
    status: Option<ExitStatus>,
}

impl CodexJob {
    pub(crate) fn new(
        child: GroupChild,
        stdout: impl Read + Send + 'static,
        stderr: impl Read + Send + 'static,
        timeout: Duration,
    ) -> Self {
        Self {
            child: Some(child),
            stdout: Some(spawn_reader(stdout, "stdout")),
            stderr: Some(spawn_reader(stderr, "stderr")),
            started: Instant::now(),
            timeout,
            status: None,
        }
    }

    /// Returns a completed result without blocking, or `None` while running.
    ///
    /// # Errors
    ///
    /// Returns [`CodexError`] for timeout, process, or bounded-output failures.
    pub fn poll(&mut self) -> Option<Result<String, CodexError>> {
        if self.started.elapsed() >= self.timeout {
            return Some(self.timeout_now());
        }
        if self.status.is_none() {
            let status = match self.child.as_mut()?.try_wait() {
                Ok(status) => status,
                Err(error) => {
                    return Some(Err(io_error("poll", "codex", error)));
                }
            };
            self.status = status;
        }
        if !self.readers_finished() {
            return None;
        }
        let status = self.status.take()?;
        self.child.take();
        Some(self.finish(status))
    }

    /// Blocks for completion with the configured timeout.
    ///
    /// # Errors
    ///
    /// Returns [`CodexError`] if the process times out or returns invalid output.
    pub fn wait(mut self) -> Result<String, CodexError> {
        let status = {
            let child = self
                .child
                .as_mut()
                .ok_or(CodexError::ReaderThread("process"))?;
            match child
                .inner()
                .wait_timeout(self.timeout)
                .map_err(|error| io_error("wait for", "codex", error))?
            {
                Some(status) => status,
                None => return self.timeout_now(),
            }
        };
        self.child.take();
        self.finish(status)
    }

    /// Terminates and reaps the whole Codex process group.
    ///
    /// # Errors
    ///
    /// Returns [`CodexError`] if termination, reaping, or reader cleanup fails.
    pub fn cancel(mut self) -> Result<(), CodexError> {
        self.terminate()?;
        let _stdout = join_reader(self.stdout.take(), "stdout")?;
        let _stderr = join_reader(self.stderr.take(), "stderr")?;
        Ok(())
    }

    fn timeout_now(&mut self) -> Result<String, CodexError> {
        match self.terminate() {
            Ok(()) => Err(CodexError::Timeout(self.timeout)),
            Err(cleanup) => Err(CodexError::OperationAndCleanup {
                primary: CodexError::Timeout(self.timeout).to_string(),
                cleanup: cleanup.to_string(),
            }),
        }
    }

    fn terminate(&mut self) -> Result<(), CodexError> {
        let Some(mut child) = self.child.take() else {
            return Ok(());
        };
        let running = child
            .try_wait()
            .map_err(|error| io_error("poll before cancelling", "codex", error))?
            .is_none();
        if running {
            child
                .kill()
                .map_err(|error| io_error("cancel", "codex process group", error))?;
        }
        child
            .wait()
            .map_err(|error| io_error("reap", "codex process group", error))?;
        Ok(())
    }

    fn readers_finished(&self) -> bool {
        self.stdout.as_ref().is_some_and(JoinHandle::is_finished)
            && self.stderr.as_ref().is_some_and(JoinHandle::is_finished)
    }

    fn finish(&mut self, status: ExitStatus) -> Result<String, CodexError> {
        let stdout = join_reader(self.stdout.take(), "stdout")?;
        let stderr = join_reader(self.stderr.take(), "stderr")?;
        if !status.success() {
            return Err(CodexError::Exit {
                code: status.code(),
                stderr: stderr.trim().to_owned(),
            });
        }
        let note = stdout.trim().to_owned();
        if note.is_empty() {
            return Err(CodexError::EmptyOutput);
        }
        Ok(note)
    }
}

fn spawn_reader(
    mut reader: impl Read + Send + 'static,
    stream: &'static str,
) -> JoinHandle<Result<String, CodexError>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        reader
            .by_ref()
            .take((MAX_OUTPUT_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| io_error("read", stream, error))?;
        if bytes.len() > MAX_OUTPUT_BYTES {
            return Err(CodexError::OutputTooLarge {
                stream,
                limit: MAX_OUTPUT_BYTES,
            });
        }
        String::from_utf8(bytes).map_err(|_| CodexError::InvalidUtf8(stream))
    })
}

fn join_reader(
    handle: Option<JoinHandle<Result<String, CodexError>>>,
    stream: &'static str,
) -> Result<String, CodexError> {
    handle
        .ok_or(CodexError::ReaderThread(stream))?
        .join()
        .map_err(|_| CodexError::ReaderThread(stream))?
}

pub(crate) fn cleanup_launch_failure(mut child: GroupChild, primary: CodexError) -> CodexError {
    let cleanup = child.kill().and_then(|()| child.wait());
    match cleanup {
        Ok(_) => primary,
        Err(error) => CodexError::OperationAndCleanup {
            primary: primary.to_string(),
            cleanup: error.to_string(),
        },
    }
}
