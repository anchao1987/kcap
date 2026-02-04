use anyhow::{Context, Result};
use std::fs::File;
use std::io::{self, Read, Write};

pub fn write_stream<R: Read>(mut reader: R, output: &str) -> Result<()> {
    // Stream capture bytes directly to stdout or a file.
    if output == "-" {
        // Allow piping to other tools without an intermediate file.
        let mut stdout = io::stdout().lock();
        io::copy(&mut reader, &mut stdout).context("failed to write to stdout")?;
        stdout.flush().ok();
        return Ok(());
    }

    // Create or truncate the output file for a single session.
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
