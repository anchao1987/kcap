use clap::{Parser, ValueEnum};

#[derive(Parser, Debug, Clone)]
#[command(
    name = "kcap",
    version,
    about = "Remote packet capture helper",
    long_about = "Capture packets remotely over SSH, or inside a Kubernetes pod via kubectl exec."
)]
/// CLI arguments for a capture run.
pub struct Args {
    // SSH identity for connecting to the target.
    #[arg(long, help = "SSH username (optional)")]
    pub ssh_user: Option<String>,

    // Direct SSH target, bypassing k8s lookup.
    #[arg(long, help = "Direct SSH target host (skip k8s)")]
    pub ssh_host: Option<String>,

    #[arg(long, default_value_t = 22, help = "SSH port")]
    pub ssh_port: u16,

    #[arg(long, help = "SSH jump host (bastion)")]
    pub jump_host: Option<String>,

    #[arg(long, help = "Kubernetes namespace")]
    pub namespace: Option<String>,

    #[arg(long, help = "Kubernetes pod name (resolved to node)")]
    pub pod: Option<String>,

    #[arg(long, help = "Container name (kubectl exec target)")]
    pub container: Option<String>,

    #[arg(long, help = "Port filter")]
    pub port: Option<u16>,

    #[arg(long, value_enum, default_value_t = Protocol::All, help = "Protocol filter")]
    pub protocol: Protocol,

    #[arg(long, default_value = "any", help = "Capture interface (e.g. eth0/any)")]
    pub iface: String,

    #[arg(long, default_value = "capture.pcap", help = "Output file, use - for stdout")]
    pub output: String,

    // Format controls which capture tool is selected remotely.
    #[arg(long, value_enum, default_value_t = CaptureFormat::Pcap, help = "Output format")]
    pub format: CaptureFormat,

    #[arg(long, help = "Capture duration (seconds), empty means run until stopped")]
    pub duration: Option<u64>,

    #[arg(long, help = "Additional capture filter expression (combined with port)")]
    pub filter: Option<String>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
/// Protocol selector for capture filtering.
pub enum Protocol {
    Tcp,
    Udp,
    All,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
/// Output format selector for capture tools.
pub enum CaptureFormat {
    Pcap,
    Pcapng,
}
