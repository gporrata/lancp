# lancp

lancp (LAN copy) is a Rust CLI for transferring files between machines on the same local network with no configuration required. It automatically discovers peers on the network and provides an interactive interface for selecting a destination.

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
lancp snd [paths...]
```

lancp will discover hosts on the LAN, show an interactive list, and begin transferring once you select a destination.

## Commands

### `lancp snd [paths...]`

Discovers hosts on the current LAN, shows them interactively, and copies files to the selected host.

- `--port` — announcement port (default: 5300)
- `--data-port` — transfer port (default: 5301)

Each path may be a file or directory:
- Files are sent as-is.
- Directories are traversed recursively. Hidden entries (files or directories beginning with `.`) are skipped entirely.

### `lancp rcv`

Listens for incoming transfers and writes received files to the current directory. Displays live transfer status per sending host.

- `--port` — announcement port to listen on (default: 5300)
- `--data-port` — transfer port to listen on (default: 5301)
