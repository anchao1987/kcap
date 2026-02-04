pub mod capture;
pub mod cli;
pub mod filter;
pub mod k8s;
pub mod output;
pub mod ssh;

use anyhow::{bail, Context, Result};
use cli::{Args, CaptureFormat};
use k8s::Target;
use tracing::{info, warn};

pub fn run(args: Args) -> Result<()> {
    // 一次抓包执行的主编排流程。
    let runner = k8s::SystemRunner;

    // 尽早解析出明确的目标主机，避免只完成部分步骤。
    let target = resolve_target(&args, &runner)?;
    let filter = filter::build_filter(args.port, args.protocol, args.filter.as_deref());

    let tool = capture::select_tool(args.format);
    // 当工具无法满足所需格式时提前告警。
    if args.format == CaptureFormat::Pcapng && tool == capture::CaptureTool::Tcpdump {
        warn!("pcapng requested but tcpdump selected; output will be pcap");
    }

    // 生成单条远程命令，将抓包数据流输出到 stdout。
    let remote_cmd = capture::build_capture_command(tool, &args.iface, args.format, filter.as_deref());
    info!(%remote_cmd, "remote capture command");

    let ssh_args = ssh::build_ssh_args(
        args.ssh_user.as_deref(),
        &target.host,
        args.ssh_port,
        args.jump_host.as_deref(),
        &remote_cmd,
    );

    let mut child = ssh::spawn_ssh(&ssh_args)?;

    let duration = args.duration;
    // 提供时长时限制抓包时间，避免会话失控。
    if let Some(d) = duration {
        capture::kill_after(&mut child, d);
    }

    let stdout = child.stdout.take().context("failed to capture ssh stdout")?;
    output::write_stream(stdout, &args.output)
        .with_context(|| format!("failed to write output to {}", args.output))?;

    let status = child.wait()?;
    if !status.success() {
        bail!("remote command failed with status {status}");
    }

    Ok(())
}

fn resolve_target(args: &Args, runner: &impl k8s::Runner) -> Result<Target> {
    // 确定实际执行抓包命令的单一主机。
    if let Some(host) = &args.ssh_host {
        return Ok(Target { host: host.clone() });
    }

    if let Some(pod) = &args.pod {
        // 将 pod 解析为节点，确保 SSH 到真实流量所在主机。
        let ns = args.namespace.as_deref().unwrap_or("default");
        let node = k8s::resolve_pod_node(runner, ns, pod)?;
        return Ok(Target { host: node });
    }

    bail!("no target specified: set --ssh-host or --pod");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::k8s::{FakeRunner, Runner};

    #[test]
    fn resolve_target_prefers_ssh_host() {
        let args = Args {
            ssh_user: None,
            ssh_host: Some("10.0.0.1".to_string()),
            ssh_port: 22,
            jump_host: None,
            namespace: None,
            pod: Some("p1".to_string()),
            container: None,
            port: None,
            protocol: cli::Protocol::All,
            iface: "any".to_string(),
            output: "capture.pcap".to_string(),
            format: cli::CaptureFormat::Pcap,
            duration: None,
            filter: None,
        };

        let runner = FakeRunner::default();
        let target = resolve_target(&args, &runner).unwrap();
        assert_eq!(target.host, "10.0.0.1");
    }

    #[test]
    fn resolve_target_from_pod() {
        let args = Args {
            ssh_user: None,
            ssh_host: None,
            ssh_port: 22,
            jump_host: None,
            namespace: Some("prod".to_string()),
            pod: Some("orders".to_string()),
            container: None,
            port: None,
            protocol: cli::Protocol::All,
            iface: "any".to_string(),
            output: "capture.pcap".to_string(),
            format: cli::CaptureFormat::Pcap,
            duration: None,
            filter: None,
        };

        let runner = FakeRunner::new("node-1");
        let target = resolve_target(&args, &runner).unwrap();
        assert_eq!(target.host, "node-1");

        let called = runner.last_command.lock().unwrap().clone();
        assert_eq!(called.program, "kubectl");
        assert!(called.args.iter().any(|a| a == "pod"));
    }
}
