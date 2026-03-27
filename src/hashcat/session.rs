use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, Result};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::select;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use super::error_parser::{ClassifiedMessage, Severity, classify_line};

/// Prefix for all agent-created hashcat session names.
const SESSION_PREFIX: &str = "attack-";

/// Events emitted by a running hashcat session.
#[derive(Debug)]
pub enum SessionEvent {
    /// A classified message from stdout or stderr.
    Message(ClassifiedMessage),
    /// A JSON status update from `--status-json`.
    Status(serde_json::Value),
    /// The process has exited.
    Exit { code: i32 },
}

/// A managed hashcat subprocess.
pub struct Session {
    binary_path: PathBuf,
    session_name: String,
    args: Vec<String>,
}

impl Session {
    /// Create a new hashcat session.
    pub fn new(binary_path: PathBuf, task_id: i64, args: Vec<String>) -> Self {
        let session_name = format!("{SESSION_PREFIX}{task_id}");
        Self {
            binary_path,
            session_name,
            args,
        }
    }

    /// The session name used for hashcat's `--session` flag.
    pub fn session_name(&self) -> &str {
        &self.session_name
    }

    /// Start the hashcat process and stream events through the returned channel.
    ///
    /// The process is killed when `cancel` is triggered.
    pub async fn start(
        &self,
        cancel: CancellationToken,
    ) -> Result<(mpsc::Receiver<SessionEvent>, Child)> {
        let mut cmd = Command::new(&self.binary_path);
        cmd.args(&self.args)
            .arg("--session")
            .arg(&self.session_name)
            .arg("--status-json")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        debug!(
            binary = %self.binary_path.display(),
            session = %self.session_name,
            "starting hashcat"
        );

        let mut child = cmd.spawn().context("failed to start hashcat")?;

        let stdout = child.stdout.take().context("missing stdout")?;
        let stderr = child.stderr.take().context("missing stderr")?;

        let (tx, rx) = mpsc::channel(256);

        // Spawn stdout reader
        let stdout_tx = tx.clone();
        let stdout_cancel = cancel.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            loop {
                select! {
                    () = stdout_cancel.cancelled() => break,
                    result = lines.next_line() => {
                        match result {
                            Ok(Some(line)) => {
                                handle_stdout_line(&line, &stdout_tx).await;
                            }
                            Ok(None) => break,
                            Err(e) => {
                                warn!(error = %e, "error reading hashcat stdout");
                                break;
                            }
                        }
                    }
                }
            }
        });

        // Spawn stderr reader
        let stderr_tx = tx.clone();
        let stderr_cancel = cancel.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            loop {
                select! {
                    () = stderr_cancel.cancelled() => break,
                    result = lines.next_line() => {
                        match result {
                            Ok(Some(line)) => {
                                if let Some(msg) = classify_line(&line) {
                                    let _ = stderr_tx.send(SessionEvent::Message(msg)).await;
                                }
                            }
                            Ok(None) => break,
                            Err(e) => {
                                warn!(error = %e, "error reading hashcat stderr");
                                break;
                            }
                        }
                    }
                }
            }
        });

        Ok((rx, child))
    }

    /// Resolve the directory where hashcat stores session files (.log, .pid, .restore).
    pub fn session_dir(binary_path: &Path) -> PathBuf {
        #[cfg(unix)]
        {
            if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
                let p = PathBuf::from(xdg).join("hashcat/sessions");
                if p.exists() {
                    return p;
                }
            }
            if let Some(home) = dirs_next_home() {
                let xdg_default = home.join(".local/share/hashcat/sessions");
                if xdg_default.exists() {
                    return xdg_default;
                }
                let legacy = home.join(".hashcat/sessions");
                if legacy.exists() {
                    return legacy;
                }
            }
        }

        #[cfg(windows)]
        {
            // On Windows, session files live next to the hashcat binary.
            if let Some(parent) = binary_path.parent() {
                return parent.to_path_buf();
            }
        }

        #[cfg(not(windows))]
        let _ = binary_path; // suppress unused warning on non-windows

        PathBuf::from(".")
    }

    /// Remove orphaned session files matching the agent prefix.
    pub fn cleanup_orphaned_sessions(session_dir: &Path) -> Result<()> {
        if !session_dir.is_dir() {
            return Ok(());
        }

        let entries = std::fs::read_dir(session_dir).context("failed to read session directory")?;

        for entry in entries {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if name_str.starts_with(SESSION_PREFIX) {
                let path = entry.path();
                if let Err(e) = std::fs::remove_file(&path) {
                    if e.kind() != std::io::ErrorKind::NotFound {
                        warn!(path = %path.display(), error = %e, "failed to remove orphaned session file");
                    }
                } else {
                    debug!(path = %path.display(), "removed orphaned session file");
                }
            }
        }

        Ok(())
    }
}

async fn handle_stdout_line(line: &str, tx: &mpsc::Sender<SessionEvent>) {
    // Try to parse as JSON status first
    let bytes = line.as_bytes();
    if serde_json::from_slice::<serde_json::Value>(bytes).is_ok() {
        if let Ok(value) = serde_json::from_str(line) {
            let _ = tx.send(SessionEvent::Status(value)).await;
            return;
        }
    }

    // Otherwise classify as a text message (hashcat routes warnings to stdout)
    if let Some(msg) = classify_line(line) {
        if matches!(msg.severity, Severity::Warning | Severity::Error) {
            let _ = tx.send(SessionEvent::Message(msg)).await;
        }
    }
}

#[cfg(unix)]
fn dirs_next_home() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}
