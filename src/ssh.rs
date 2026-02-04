use anyhow::{Context, Result};
use std::process::{Child, Command, Stdio};

/// Builds SSH arguments for running a remote command.
/// Parameters: `user` (Option<&str>) optional SSH username.
/// Parameters: `host` (&str) target host.
/// Parameters: `port` (u16) SSH port.
/// Parameters: `jump_host` (Option<&str>) optional bastion host.
/// Parameters: `remote_cmd` (&str) command executed on the remote host.
/// Returns: Vec<String> argument list for the ssh command.
pub fn build_ssh_args(
    user: Option<&str>,
    host: &str,
    port: u16,
    jump_host: Option<&str>,
    remote_cmd: &str,
) -> Vec<String> {
    // Build a non-interactive SSH invocation for remote capture.
    let mut args = Vec::new();
    args.push("-o".to_string());
    args.push("BatchMode=yes".to_string());
    args.push("-p".to_string());
    args.push(port.to_string());

    if let Some(jump) = jump_host {
        // Support bastion jumps without changing caller logic.
        args.push("-J".to_string());
        args.push(jump.to_string());
    }

    let target = match user {
        Some(u) => format!("{u}@{host}"),
        None => host.to_string(),
    };
    args.push(target);

    args.push("--".to_string());
    args.push("sh".to_string());
    args.push("-c".to_string());
    args.push(remote_cmd.to_string());

    args
}

/// Spawns an ssh process with piped stdout.
/// Parameters: `args` (&[String]) argument list for ssh.
/// Returns: Result<Child> handle to the spawned process.
pub fn spawn_ssh(args: &[String]) -> Result<Child> {
    // Keep stdout piped for capture bytes and inherit stderr for feedback.
    Command::new("ssh")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context("failed to spawn ssh")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_args_basic() {
        let args = build_ssh_args(Some("root"), "10.0.0.1", 2222, None, "echo hi");
        assert_eq!(args[0], "-o");
        assert!(args.iter().any(|a| a == "root@10.0.0.1"));
        assert!(args.iter().any(|a| a == "-c"));
        assert!(args.iter().any(|a| a == "echo hi"));
    }

    #[test]
    fn ssh_args_jump() {
        let args = build_ssh_args(None, "host", 22, Some("jump"), "cmd");
        assert!(args.iter().any(|a| a == "-J"));
        assert!(args.iter().any(|a| a == "jump"));
    }
}
