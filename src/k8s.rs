use anyhow::{bail, Context, Result};
use std::process::{Command, Stdio};
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Target {
    pub host: String,
}

pub trait Runner {
    fn run_capture(&self, program: &str, args: &[&str]) -> Result<String>;
}

pub struct SystemRunner;

impl Runner for SystemRunner {
    fn run_capture(&self, program: &str, args: &[&str]) -> Result<String> {
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

pub fn resolve_pod_node(runner: &impl Runner, namespace: &str, pod: &str) -> Result<String> {
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
pub struct FakeRunner {
    pub node_name: String,
    pub last_command: Mutex<CommandRecord>,
}

#[derive(Debug, Default, Clone)]
pub struct CommandRecord {
    pub program: String,
    pub args: Vec<String>,
}

impl FakeRunner {
    pub fn new(node_name: &str) -> Self {
        Self {
            node_name: node_name.to_string(),
            last_command: Mutex::new(CommandRecord::default()),
        }
    }
}

impl Runner for FakeRunner {
    fn run_capture(&self, program: &str, args: &[&str]) -> Result<String> {
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
