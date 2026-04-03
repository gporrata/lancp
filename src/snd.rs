use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use walkdir::WalkDir;

use crate::discovery;

pub async fn run(paths: Vec<PathBuf>, port: u16, data_port: u16) -> Result<()> {
    println!("Discovering hosts on LAN...");
    let hosts = discovery::discover(port, 3).await?;

    if hosts.is_empty() {
        anyhow::bail!("No hosts found. Make sure the receiver is running `lancp rcv`.");
    }

    let host_names: Vec<String> = hosts.iter().map(|h| h.to_string()).collect();
    let selection = inquire::Select::new("Select a host:", host_names.clone())
        .prompt()
        .context("No host selected")?;
    let idx = host_names.iter().position(|s| s == &selection).unwrap();
    let host = &hosts[idx];

    let files = collect_files(&paths)?;
    if files.is_empty() {
        anyhow::bail!("No files to send.");
    }

    println!("Connecting to {}...", host);
    let mut stream = TcpStream::connect((host.addr, data_port))
        .await
        .context("Failed to connect to host")?;

    // Send file count
    stream.write_u32(files.len() as u32).await?;

    let mp = MultiProgress::new();
    let total_bytes: u64 = files.iter().map(|(_, _, size)| size).sum();
    let overall = mp.add(ProgressBar::new(total_bytes));
    overall.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{wide_bar}] {bytes}/{total_bytes} ({eta})")?
            .progress_chars("=> "),
    );
    overall.set_message("Total");

    for (abs_path, rel_path, size) in &files {
        let rel_bytes = rel_path.as_bytes();
        stream.write_u16(rel_bytes.len() as u16).await?;
        stream.write_all(rel_bytes).await?;
        stream.write_u64(*size).await?;

        let file = tokio::fs::File::open(abs_path)
            .await
            .with_context(|| format!("Failed to open {}", abs_path.display()))?;
        let mut reader = tokio::io::BufReader::new(file);

        let file_bar = mp.add(ProgressBar::new(*size));
        file_bar.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{wide_bar}] {bytes}/{total_bytes}")?
                .progress_chars("=> "),
        );
        file_bar.set_message(
            abs_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
        );

        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = reader.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            stream.write_all(&buf[..n]).await?;
            file_bar.inc(n as u64);
            overall.inc(n as u64);
        }
        file_bar.finish_and_clear();
    }

    overall.finish_with_message("Done");
    println!("Transfer complete.");
    Ok(())
}

/// Returns (absolute_path, relative_path, size) for every non-hidden file under `paths`.
fn collect_files(paths: &[PathBuf]) -> Result<Vec<(PathBuf, String, u64)>> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_file() {
            let size = std::fs::metadata(path)?.len();
            let rel = path
                .file_name()
                .context("File has no name")?
                .to_string_lossy()
                .into_owned();
            files.push((path.clone(), rel, size));
        } else if path.is_dir() {
            let base = path.parent().unwrap_or(Path::new("."));
            for entry in WalkDir::new(path).into_iter().filter_entry(|e| !is_hidden(e)) {
                let entry = entry?;
                if entry.file_type().is_file() {
                    let size = entry.metadata()?.len();
                    let rel = entry
                        .path()
                        .strip_prefix(base)
                        .unwrap_or(entry.path())
                        .to_string_lossy()
                        .into_owned();
                    files.push((entry.path().to_path_buf(), rel, size));
                }
            }
        }
    }
    Ok(files)
}

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}
