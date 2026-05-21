# vector-tunnel-agent

Linux user-space daemon that hosts a Microsoft Dev Tunnel and serves PTY
shells to the [Vector](https://github.com/colligo/vector) macOS terminal.

Sign in once, leave the agent running, and Vector connects via the Microsoft
relay. No incoming firewall ports. No VS Code. No browser after first run.

## Install (Debian / Ubuntu, amd64)

```sh
VERSION=2026.5.10
wget "https://github.com/colligo/vector/releases/latest/download/vector-tunnel-agent_${VERSION}_amd64.deb"
sudo apt install "./vector-tunnel-agent_${VERSION}_amd64.deb"
```

## Install (Debian / Ubuntu, arm64)

```sh
VERSION=2026.5.10
wget "https://github.com/colligo/vector/releases/latest/download/vector-tunnel-agent_${VERSION}_arm64.deb"
sudo apt install "./vector-tunnel-agent_${VERSION}_arm64.deb"
```

## Run

```sh
vector-tunnel-agent
```

First run prints a device-code URL plus a short code. Open the URL in any
browser, paste the code, sign in with GitHub or Microsoft. The token is
cached at `~/.config/vector/agent-token` (mode 0600) for subsequent runs.

## Re-authenticate

```sh
vector-tunnel-agent --reauth
```

Wipes the cached token and re-runs the device-code flow.

## Status

```sh
vector-tunnel-agent --status
```

Prints the cached provider and token expiry. Does not query the live
Dev Tunnels Management API.

## Persistence

v1 is manual-run only (D-02). No systemd unit. To survive SSH disconnect,
launch under `tmux` or `nohup`:

```sh
tmux new -d -s vector-agent vector-tunnel-agent
```

To reattach:

```sh
tmux attach -t vector-agent
```

## Connect from Vector on macOS

After the agent registers a tunnel (label `vector-agent`, name
`vector-{hostname}`), open Vector on your Mac:

1. `Cmd-Shift-T` opens the picker.
2. Select your machine.
3. A new tab opens with a remote shell.

## Out of scope (v1.x)

- systemd auto-start / user service unit
- rpm / yum / dnf packaging
- snap, flatpak, AppImage
- Windows host (.msi / chocolatey)
- arbitrary SSH targets — agent is Dev-Tunnels only

## Build from source

```sh
cargo install cargo-deb
cargo build --release -p vector-tunnel-agent
cargo deb -p vector-tunnel-agent --no-build
```

The .deb lands in `target/debian/`.

## License

MIT. See the top-level `LICENSE` in the repo.
