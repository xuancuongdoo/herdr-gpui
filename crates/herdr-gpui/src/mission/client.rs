//! Blocking missiond UDS client (res-03 step a).
//! Own HTTP/1.1 over `std::os::unix::net::UnixStream`.
//! Never uses the herdr socket worker.

use std::io::{Read, Write};
use std::path::PathBuf;

/// Default missiond socket: `~/.herdr/missiond.sock`.
/// Override with non-empty `HERDR_MISSIOND_SOCK`.
pub fn socket_path() -> PathBuf {
    if let Ok(p) = std::env::var("HERDR_MISSIOND_SOCK")
        && !p.trim().is_empty()
    {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    std::path::Path::new(&home)
        .join(".herdr")
        .join("missiond.sock")
}

/// Fetch and parse `GET /missions/<mid>/graph` agents.
pub fn fetch_graph(mid: &str) -> anyhow::Result<Vec<super::graph::MissionAgent>> {
    if mid.is_empty() || mid.contains('/') {
        anyhow::bail!("invalid mission id");
    }
    let sock = socket_path();
    let body = get_graph_body(&sock, mid)?;
    let agents = super::graph::parse_graph(&body)?;
    Ok(agents)
}

fn get_graph_body(sock: &std::path::Path, mid: &str) -> anyhow::Result<String> {
    use std::os::unix::net::UnixStream;

    let mut stream = UnixStream::connect(sock)
        .map_err(|e| anyhow::anyhow!("connect {}: {e}", sock.display()))?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .ok();
    let req = format!(
        "GET /missions/{mid}/graph HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|e| anyhow::anyhow!("write {}: {e}", sock.display()))?;
    let mut buf: Vec<u8> = Vec::new();
    stream
        .read_to_end(&mut buf)
        .map_err(|e| anyhow::anyhow!("read {}: {e}", sock.display()))?;
    let text = String::from_utf8_lossy(&buf).into_owned();
    let Some(sep) = text.find("\r\n\r\n") else {
        anyhow::bail!("daemon returned malformed HTTP response");
    };
    let (head, body) = text.split_at(sep + 4);
    let status_line = head.lines().next().unwrap_or("");
    let code: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let body = if is_chunked(head) {
        decode_chunked(body)?
    } else {
        body.to_string()
    };
    if (200..300).contains(&code) {
        Ok(body)
    } else {
        anyhow::bail!("missiond GET graph {mid} -> {code}: {}", body.trim())
    }
}

fn is_chunked(head: &str) -> bool {
    head.lines().skip(1).any(|l| {
        let l = l.to_ascii_lowercase();
        l.starts_with("transfer-encoding:") && l.contains("chunked")
    })
}

fn decode_chunked(body: &str) -> anyhow::Result<String> {
    let bytes = body.as_bytes();
    let mut out: Vec<u8> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let Some(eol) = find_crlf(bytes, i) else {
            anyhow::bail!("malformed chunked body");
        };
        let size_str = std::str::from_utf8(&bytes[i..eol])
            .map_err(|_| anyhow::anyhow!("malformed chunk size"))?;
        let size_str = size_str.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(size_str, 16)
            .map_err(|_| anyhow::anyhow!("malformed chunk size"))?;
        i = eol + 2;
        if size == 0 {
            break;
        }
        if i + size > bytes.len() {
            anyhow::bail!("malformed chunked body");
        }
        out.extend_from_slice(&bytes[i..i + size]);
        i += size;
        if bytes.get(i..i + 2) == Some(b"\r\n") {
            i += 2;
        }
    }
    String::from_utf8(out).map_err(|e| anyhow::anyhow!("chunked body not utf8: {e}"))
}

fn find_crlf(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    while i + 1 < bytes.len() {
        if bytes[i] == b'\r' && bytes[i + 1] == b'\n' {
            return Some(i);
        }
        i += 1;
    }
    None
}
