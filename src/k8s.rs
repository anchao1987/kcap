use anyhow::{bail, Context, Result};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

#[derive(Debug, Clone)]
/// Resolved target for running remote capture.
pub enum Target {
    Ssh { host: String },
    KubernetesExec {
        namespace: String,
        pod: String,
        container: Option<String>,
    },
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

/// Builds kubectl exec arguments for running a remote command inside a pod.
/// Parameters: `namespace` (&str) pod namespace.
/// Parameters: `pod` (&str) pod name.
/// Parameters: `container` (Option<&str>) container name.
/// Parameters: `remote_cmd` (&str) command executed inside the container.
/// Returns: Vec<String> argument list for kubectl.
pub fn build_kubectl_exec_args(
    namespace: &str,
    pod: &str,
    container: Option<&str>,
    remote_cmd: &str,
) -> Vec<String> {
    let mut args = Vec::new();
    args.push("exec".to_string());
    args.push("-n".to_string());
    args.push(namespace.to_string());
    args.push(pod.to_string());

    if let Some(c) = container {
        args.push("-c".to_string());
        args.push(c.to_string());
    }

    // Execute through a shell to preserve the capture command quoting.
    args.push("--".to_string());
    args.push("sh".to_string());
    args.push("-c".to_string());
    args.push(remote_cmd.to_string());
    args
}

/// Spawns a kubectl exec process with piped stdout.
/// Parameters: `args` (&[String]) argument list for kubectl exec.
/// Returns: Result<Child> handle to the spawned process.
pub fn spawn_kubectl_exec(args: &[String]) -> Result<Child> {
    Command::new("kubectl")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to spawn kubectl exec")
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

    #[test]
    fn kubectl_exec_args_basic() {
        let args = build_kubectl_exec_args("prod", "orders", None, "tcpdump -i any -w -");
        assert_eq!(args[0], "exec");
        assert!(args.iter().any(|a| a == "prod"));
        assert!(args.iter().any(|a| a == "orders"));
        assert!(args.iter().any(|a| a == "sh"));
        assert!(args.iter().any(|a| a == "tcpdump -i any -w -"));

        let sh_index = args.iter().position(|a| a == "sh").unwrap();
        let c_index = args.iter().position(|a| a == "-c").unwrap();
        assert_eq!(c_index, sh_index + 1);
    }

    #[test]
    fn kubectl_exec_args_container() {
        let args = build_kubectl_exec_args("prod", "orders", Some("api"), "cmd");
        assert!(args.iter().any(|a| a == "-c"));
        assert!(args.iter().any(|a| a == "api"));
    }
}
