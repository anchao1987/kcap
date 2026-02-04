use anyhow::{bail, Context, Result};
use std::process::{Command, Stdio};
use std::sync::Mutex;

#[derive(Debug, Clone)]
/// Resolved target host for running remote capture.
pub struct Target {
    pub host: String,
}

/// Executes external commands for capture-related queries.
pub trait Runner {
    /// Runs a command and returns trimmed stdout on success.
    /// Parameters: `program` (&str) executable name.
    /// Parameters: `args` (&[&str]) argument list.
    /// Returns: Result<String> with trimmed stdout or an error.
    // Abstract external command execution for testability.
    fn run_capture(&self, program: &str, args: &[&str]) -> Result<String>;
}

/// Runner implementation that invokes system binaries.
pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run_capture(&self, program: &str, args: &[&str]) -> Result<String> {
        // Capture stdout and stderr to surface kubectl errors clearly.
        let output = Command::new(program)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .with_context(|| format!("failed to run {program}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("{program} failed: {stderr}");
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

/// Resolves a Kubernetes pod to its node name.
/// Parameters: `runner` (&impl Runner) command runner.
/// Parameters: `namespace` (&str) pod namespace.
/// Parameters: `pod` (&str) pod name.
/// Returns: Result<String> node name or an error if missing.
pub fn resolve_pod_node(runner: &impl Runner, namespace: &str, pod: &str) -> Result<String> {
    // Map pod to node to determine where capture should run.
    let args = [
        "get",
        "pod",
        pod,
        "-n",
        namespace,
        "-o",
        "jsonpath={.spec.nodeName}",
    ];
    let node = runner.run_capture("kubectl", &args)?;
    if node.is_empty() {
        bail!("pod {pod} has no nodeName");
    }
    Ok(node)
}

#[derive(Debug, Default)]
/// Test runner that returns a fixed node and records calls.
pub struct FakeRunner {
    pub node_name: String,
    pub last_command: Mutex<CommandRecord>,
}

#[derive(Debug, Default, Clone)]
/// Records the last command executed by a runner.
pub struct CommandRecord {
    pub program: String,
    pub args: Vec<String>,
}

impl FakeRunner {
    /// Creates a FakeRunner that returns the provided node name.
    /// Parameters: `node_name` (&str) node name to return.
    /// Returns: FakeRunner instance for tests.
    pub fn new(node_name: &str) -> Self {
        Self {
            node_name: node_name.to_string(),
            last_command: Mutex::new(CommandRecord::default()),
        }
    }
}

impl Runner for FakeRunner {
    fn run_capture(&self, program: &str, args: &[&str]) -> Result<String> {
        // Record calls so tests don't need to execute kubectl.
        let mut rec = self.last_command.lock().unwrap();
        rec.program = program.to_string();
        rec.args = args.iter().map(|s| s.to_string()).collect();
        Ok(self.node_name.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_pod_node_uses_kubectl() {
        let runner = FakeRunner::new("node-x");
        let node = resolve_pod_node(&runner, "prod", "orders").unwrap();
        assert_eq!(node, "node-x");

        let rec = runner.last_command.lock().unwrap().clone();
        assert_eq!(rec.program, "kubectl");
        assert!(rec.args.contains(&"get".to_string()));
        assert!(rec.args.contains(&"pod".to_string()));
        assert!(rec.args.contains(&"orders".to_string()));
    }
}
