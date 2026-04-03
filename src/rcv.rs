use std::path::{Component, Path, PathBuf};
use anyhow::{Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::discovery;

pub async fn run(data_port: u16) -> Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", data_port)).await?;

    let local_addrs = discovery::local_ipv4_addrs();
    let addr_list = if local_addrs.is_empty() {
        "unknown".to_string()
    } else {
        local_addrs.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(", ")
    };
    println!("Announcing on LAN [{}]", addr_list);
    println!("Waiting for senders on port {} — press Ctrl+C to stop.\n", data_port);

    tokio::spawn(async move {
        if let Err(e) = discovery::announce(data_port).await {
            eprintln!("Announcement error: {}", e);
        }
    });

    let mp = MultiProgress::new();

    loop {
        let (stream, addr) = listener.accept().await?;
        let mp_clone = mp.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_transfer(stream, addr, mp_clone).await {
                eprintln!("Transfer error from {}: {}", addr, e);
            }
        });
    }
}

async fn handle_transfer(
    mut stream: tokio::net::TcpStream,
    addr: std::net::SocketAddr,
    mp: MultiProgress,
) -> Result<()> {
    let file_count = stream.read_u32().await?;
    mp.println(format!("Receiving {} file(s) from {}", file_count, addr))?;

    for _ in 0..file_count {
        let path_len = stream.read_u16().await? as usize;
        let mut path_bytes = vec![0u8; path_len];
        stream.read_exact(&mut path_bytes).await?;
        let rel_str = String::from_utf8(path_bytes).context("Invalid path encoding")?;

        let file_size = stream.read_u64().await?;
        let dest = sanitize_path(&rel_str);

        if let Some(parent) = dest.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        let bar = mp.add(ProgressBar::new(file_size));
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{wide_bar}] {bytes}/{total_bytes} ({eta})")?
                .progress_chars("=> "),
        );
        bar.set_message(
            dest.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        );

        let file = tokio::fs::File::create(&dest)
            .await
            .with_context(|| format!("Failed to create {}", dest.display()))?;
        let mut writer = tokio::io::BufWriter::new(file);

        let mut remaining = file_size;
        let mut buf = vec![0u8; 64 * 1024];
        while remaining > 0 {
            let to_read = (remaining as usize).min(buf.len());
            let n = stream.read(&mut buf[..to_read]).await?;
            if n == 0 {
                anyhow::bail!("Connection closed mid-transfer");
            }
            writer.write_all(&buf[..n]).await?;
            bar.inc(n as u64);
            remaining -= n as u64;
        }
        writer.flush().await?;
        bar.finish_with_message(format!("{} done", dest.display()));
    }

    Ok(())
}

/// Strip any path components that could escape the current directory (`.`, `..`, root `/`).
fn sanitize_path(rel: &str) -> PathBuf {
    let mut result = PathBuf::new();
    for component in Path::new(rel).components() {
        if let Component::Normal(c) = component {
            result.push(c);
        }
    }
    result
}
