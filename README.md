# lancp

lancp (LAN copy) is a Rust CLI for transferring files between machines on the same local network with no configuration required. It uses mDNS to automatically discover peers and provides an interactive interface for selecting a destination.

Interactive lists use [inquire](https://github.com/mikaelmello/inquire) and live transfer status uses [indicatif](https://github.com/console-rs/indicatif).

## Installation

```sh
cargo install lancp
```

Or build from source:

```sh
git clone https://github.com/youruser/lancp
cd lancp
cargo build --release
```

## Usage

On the **receiving** machine:

```sh
lancp rcv
```

On the **sending** machine:

```sh
lancp snd file.txt photo.jpg
```

lancp will discover receivers on the LAN via mDNS, show an interactive list, and begin transferring once you select a destination.

## Commands

### `lancp snd [paths...]`

Discovers receivers on the current LAN and copies files to the selected host. No port configuration needed — the receiver's address and port are discovered automatically via mDNS.

Each path may be a file or directory:
- Files are sent as-is.
- Directories are traversed recursively. Hidden entries (files or directories beginning with `.`) are skipped unless `--hidden` is passed.

**Flags:**
- `--hidden` — include hidden files and directories (those beginning with `.`) when traversing directories.

### `lancp rcv`

Announces itself on the LAN via mDNS, listens for incoming transfers, and writes received files to the current directory. Displays live transfer status per sending host.

- `--data-port` — transfer port (default: 5301)

## How it works

Discovery uses mDNS (multicast DNS, the same protocol used by AirDrop and Bonjour) on the `_lancp._tcp.local.` service type. mDNS uses the `224.0.0.251` link-local multicast address, which all Wi-Fi access points forward unconditionally within the subnet — no firewall rules or router configuration required.

File transfer uses a direct TCP connection on the data port advertised in the mDNS record.
