use anyhow::{Context, Result};
use std::fs::File;
use std::io::{self, Read, Write};

pub fn write_stream<R: Read>(mut reader: R, output: &str) -> Result<()> {
    // 将抓包字节流直接写到 stdout 或文件。
    if output == "-" {
        // 允许无中间文件地管道输出到其他工具。
        let mut stdout = io::stdout().lock();
        io::copy(&mut reader, &mut stdout).context("failed to write to stdout")?;
        stdout.flush().ok();
        return Ok(());
    }

    // 为单次抓包创建或截断输出文件。
    let mut file = File::create(output).with_context(|| format!("failed to create {output}"))?;
    io::copy(&mut reader, &mut file).context("failed to write to file")?;
    file.flush().ok();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn write_stream_to_file() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_string_lossy().to_string();
        let data = Cursor::new(b"test-data".to_vec());
        write_stream(data, &path).unwrap();

        let mut content = String::new();
        File::open(path).unwrap().read_to_string(&mut content).unwrap();
        assert_eq!(content, "test-data");
    }
}
