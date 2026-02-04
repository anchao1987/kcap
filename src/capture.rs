use crate::cli::CaptureFormat;
use std::process::Child;
use std::thread;
use std::time::Duration;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CaptureTool {
    Tcpdump,
    Tshark,
}

pub fn select_tool(format: CaptureFormat) -> CaptureTool {
    // tshark 才能稳定输出 pcapng，pcap 则可用 tcpdump。
    match format {
        CaptureFormat::Pcapng => CaptureTool::Tshark,
        CaptureFormat::Pcap => CaptureTool::Tcpdump,
    }
}

pub fn build_capture_command(
    tool: CaptureTool,
    iface: &str,
    format: CaptureFormat,
    filter: Option<&str>,
) -> String {
    // 构建可安全执行的命令，把抓包字节流输出到 stdout。
    let filter_escaped = filter.map(shell_escape_single_quotes);

    match tool {
        CaptureTool::Tcpdump => {
            let mut cmd = format!("tcpdump -i {iface} -U -s 0");
            if let Some(f) = filter_escaped {
                cmd.push(' ');
                cmd.push_str(&f);
            }
            cmd.push_str(" -w -");
            cmd
        }
        CaptureTool::Tshark => {
            let fmt = match format {
                CaptureFormat::Pcap => "pcap",
                CaptureFormat::Pcapng => "pcapng",
            };
            let mut cmd = format!("tshark -i {iface} -q -w - -F {fmt}");
            if let Some(f) = filter_escaped {
                cmd.push_str(" -f ");
                cmd.push_str(&f);
            }
            cmd
        }
    }
}

pub fn kill_after(child: &mut Child, seconds: u64) {
    if seconds == 0 {
        return;
    }

    // 在不阻塞调用线程的情况下强制停止抓包。
    let id = child.id();
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(seconds));
        #[cfg(unix)]
        {
            let _ = std::process::Command::new("kill")
                .arg("-TERM")
                .arg(id.to_string())
                .status();
        }
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &id.to_string(), "/T", "/F"])
                .status();
        }
    });
}

fn shell_escape_single_quotes(input: &str) -> String {
    // 远程命令经 `sh -c` 执行，必须保证引号安全。
    if input.is_empty() {
        return "''".to_string();
    }
    let escaped = input.replace('\'', "'\\''");
    format!("'{escaped}'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_single_quotes() {
        let s = "host 10.0.0.1 and tcp port 443";
        assert_eq!(shell_escape_single_quotes(s), "'host 10.0.0.1 and tcp port 443'");
        let s2 = "host 'bad'";
        assert_eq!(shell_escape_single_quotes(s2), "'host '\\'''bad'\\'''".to_string());
    }

    #[test]
    fn build_tcpdump_command() {
        let cmd = build_capture_command(
            CaptureTool::Tcpdump,
            "any",
            CaptureFormat::Pcap,
            Some("tcp port 443"),
        );
        assert!(cmd.contains("tcpdump -i any"));
        assert!(cmd.contains("'tcp port 443'"));
        assert!(cmd.ends_with("-w -"));
    }

    #[test]
    fn build_tshark_command() {
        let cmd = build_capture_command(
            CaptureTool::Tshark,
            "eth0",
            CaptureFormat::Pcapng,
            Some("port 53"),
        );
        assert!(cmd.contains("tshark -i eth0"));
        assert!(cmd.contains("-F pcapng"));
        assert!(cmd.contains("-f 'port 53'"));
    }
}
