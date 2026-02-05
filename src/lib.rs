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

/// Runs a single capture workflow from parsed CLI arguments.
/// Parameters: `args` (Args) parsed CLI arguments.
/// Returns: Result<()> indicating success or failure.
pub fn run(args: Args) -> Result<()> {
    // Orchestrates a single capture run.
    // Resolve a concrete target early to avoid partial work.
    let target = resolve_target(&args)?;
    let filter = filter::build_filter(args.port, args.protocol, args.filter.as_deref());

    let tool = capture::select_tool(args.format);
    // Warn when the selected tool cannot honor the requested format.
    if args.format == CaptureFormat::Pcapng && tool == capture::CaptureTool::Tcpdump {
        warn!("pcapng requested but tcpdump selected; output will be pcap");
    }

    // Build a single remote command that streams capture bytes to stdout.
    let remote_cmd = capture::build_capture_command(tool, &args.iface, args.format, filter.as_deref());
    info!(%remote_cmd, "remote capture command");

    let mut child = match target {
        Target::Ssh { host } => {
            let ssh_args = ssh::build_ssh_args(
                args.ssh_user.as_deref(),
                &host,
                args.ssh_port,
                args.jump_host.as_deref(),
                &remote_cmd,
            );
            ssh::spawn_ssh(&ssh_args)?
        }
        Target::KubernetesExec {
            namespace,
            pod,
            container,
        } => {
            // Run capture inside the pod via kubectl exec.
            let kubectl_args =
                k8s::build_kubectl_exec_args(&namespace, &pod, container.as_deref(), &remote_cmd);
            k8s::spawn_kubectl_exec(&kubectl_args)?
        }
    };

    let duration = args.duration;
    // Bound the capture duration to avoid runaway sessions.
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

fn resolve_target(args: &Args) -> Result<Target> {
    // Choose the single host that will execute the capture command.
    if let Some(host) = &args.ssh_host {
        return Ok(Target::Ssh { host: host.clone() });
    }

    if let Some(pod) = &args.pod {
        let ns = args.namespace.as_deref().unwrap_or("default");
        // Use kubectl exec so capture runs inside the container's network namespace.
        return Ok(Target::KubernetesExec {
            namespace: ns.to_string(),
            pod: pod.clone(),
            container: args.container.clone(),
        });
    }

    bail!("no target specified: set --ssh-host or --pod");
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let target = resolve_target(&args).unwrap();
        match target {
            Target::Ssh { host } => assert_eq!(host, "10.0.0.1"),
            _ => panic!("expected ssh target"),
        }
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
        let target = resolve_target(&args).unwrap();
        match target {
            Target::KubernetesExec { namespace, pod, .. } => {
                assert_eq!(namespace, "prod");
                assert_eq!(pod, "orders");
            }
            _ => panic!("expected kubectl exec target"),
        }
    }

    #[test]
    fn resolve_target_pod_with_container() {
        let args = Args {
            ssh_user: None,
            ssh_host: None,
            ssh_port: 22,
            jump_host: None,
            namespace: Some("prod".to_string()),
            pod: Some("orders".to_string()),
            container: Some("api".to_string()),
            port: None,
            protocol: cli::Protocol::All,
            iface: "any".to_string(),
            output: "capture.pcap".to_string(),
            format: cli::CaptureFormat::Pcap,
            duration: None,
            filter: None,
        };

        let target = resolve_target(&args).unwrap();
        match target {
            Target::KubernetesExec {
                namespace,
                pod,
                container,
            } => {
                assert_eq!(namespace, "prod");
                assert_eq!(pod, "orders");
                assert_eq!(container.as_deref(), Some("api"));
            }
            _ => panic!("expected kubectl exec target"),
        }
    }
}
