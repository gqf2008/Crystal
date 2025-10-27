use std::fmt::Write;
use std::fs::File;
use std::io::{BufReader, Read};

use anyhow::{Context, Result};

const BUFFER_SIZE: usize = 8 * 1024;

pub fn client_binary_hash() -> Result<Vec<u8>> {
    let exe_path = std::env::current_exe().context("failed to determine executable path")?;
    let file = File::open(&exe_path)
        .with_context(|| format!("failed to open client binary `{}`", exe_path.display()))?;

    hash_reader(BufReader::with_capacity(BUFFER_SIZE, file))
}

fn hash_reader<R: Read>(mut reader: R) -> Result<Vec<u8>> {
    let mut hasher = md5::Context::new();
    let mut buffer = [0u8; BUFFER_SIZE];

    loop {
        let read = reader
            .read(&mut buffer)
            .context("failed to read data for hashing")?;
        if read == 0 {
            break;
        }
        hasher.consume(&buffer[..read]);
    }

    let digest = hasher.finalize();
    Ok(digest.to_vec())
}

pub fn hash_to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(&mut output, "{:02x}", byte);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn hashes_match_known_md5() {
        let data = Cursor::new(b"hello world".as_slice());
        let digest = hash_reader(data).expect("hashing should succeed");
        assert_eq!(hash_to_hex(&digest), "5eb63bbbe01eeed093cb22bb8f5acdc3");
    }
}
