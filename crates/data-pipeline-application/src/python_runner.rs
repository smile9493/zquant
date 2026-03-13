use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use tokio::io::AsyncWriteExt;

#[async_trait]
pub trait PythonRunner: Send + Sync {
    async fn run_json(
        &self,
        script_path: &Path,
        input: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value>;
}

pub struct SubprocessPythonRunner {
    python_exe: String,
    timeout: Duration,
}

impl Default for SubprocessPythonRunner {
    fn default() -> Self {
        Self {
            python_exe: "python".to_string(),
            timeout: Duration::from_secs(30),
        }
    }
}

impl SubprocessPythonRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_python_exe(mut self, python_exe: impl Into<String>) -> Self {
        self.python_exe = python_exe.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[async_trait]
impl PythonRunner for SubprocessPythonRunner {
    async fn run_json(
        &self,
        script_path: &Path,
        input: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let mut child = tokio::process::Command::new(&self.python_exe)
            .arg(script_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow::anyhow!("failed to spawn python subprocess: {}", e))?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("failed to open stdin for python subprocess"))?;
        let input_bytes = serde_json::to_vec(&input)
            .map_err(|e| anyhow::anyhow!("failed to serialize python input json: {}", e))?;

        stdin
            .write_all(&input_bytes)
            .await
            .map_err(|e| anyhow::anyhow!("failed to write to python stdin: {}", e))?;
        drop(stdin);

        let output = tokio::time::timeout(self.timeout, child.wait_with_output())
            .await
            .map_err(|_| anyhow::anyhow!("python subprocess timed out after {:?}", self.timeout))?
            .map_err(|e| anyhow::anyhow!("failed to wait for python subprocess: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "python subprocess failed (exit={:?}): {}",
                output.status.code(),
                stderr.trim()
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let value: serde_json::Value = serde_json::from_str(stdout.trim())
            .map_err(|e| anyhow::anyhow!("failed to parse python stdout as json: {}", e))?;
        Ok(value)
    }
}

