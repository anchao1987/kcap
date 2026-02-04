use clap::{Parser, ValueEnum};

#[derive(Parser, Debug, Clone)]
#[command(name = "kcap", version, about = "Remote packet capture helper")]
pub struct Args {
    #[arg(long)]
    pub ssh_user: Option<String>,

    #[arg(long)]
    pub ssh_host: Option<String>,

    #[arg(long, default_value_t = 22)]
    pub ssh_port: u16,

    #[arg(long)]
    pub jump_host: Option<String>,

    #[arg(long)]
    pub namespace: Option<String>,

    #[arg(long)]
    pub pod: Option<String>,

    #[arg(long)]
    pub container: Option<String>,

    #[arg(long)]
    pub port: Option<u16>,

    #[arg(long, value_enum, default_value_t = Protocol::All)]
    pub protocol: Protocol,

    #[arg(long, default_value = "any")]
    pub iface: String,

    #[arg(long, default_value = "capture.pcap")]
    pub output: String,

    #[arg(long, value_enum, default_value_t = CaptureFormat::Pcap)]
    pub format: CaptureFormat,

    #[arg(long)]
    pub duration: Option<u64>,

    #[arg(long)]
    pub filter: Option<String>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum Protocol {
    Tcp,
    Udp,
    All,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum CaptureFormat {
    Pcap,
    Pcapng,
}
