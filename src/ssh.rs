use anyhow::{Context, Result};
use std::process::{Child, Command, Stdio};

pub fn build_ssh_args(
    user: Option<&str>,
    host: &str,
    port: u16,
    jump_host: Option<&str>,
    remote_cmd: &str,
) -> Vec<String> {
    // 构造非交互式 SSH 调用用于远程抓包。
    let mut args = Vec::new();
    args.push("-o".to_string());
    args.push("BatchMode=yes".to_string());
    args.push("-p".to_string());
    args.push(port.to_string());

    if let Some(jump) = jump_host {
        // 支持堡垒机跳转且不改变调用方逻辑。
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

pub fn spawn_ssh(args: &[String]) -> Result<Child> {
    // stdout 保持管道输出抓包数据，stderr 继承以便提示用户。
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
