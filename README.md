# Census

The fetch tool that already counted.

A daemon, `censusd`, collects your system's facts ahead of time. `census` asks it over a Unix socket and prints the answer in about a millisecond. On SeganOS it also knows the kernel profile, the audio latency and the compositor; everywhere else it shows what is there.

![Census on a wide terminal](./docs/shots/classic-wide.png)

## What it prints

A logo on the left and an aligned list on the right, in four groups: System, Hardware, Session, Audio. Labels are dim, values are your terminal's own foreground, and one accent (your theme's magenta) marks the logo and the section names. No boxes, no bars.

On a narrow terminal the logo goes; on a short one the least important lines go.

| Narrow | Short |
|---|---|
| ![Narrow](./docs/shots/classic-narrow.png) | ![Short](./docs/shots/classic-short.png) |

`census --layout compact` puts a small logo above a list with thin rules between the groups:

![Compact](./docs/shots/compact.png)

`census --layout dashboard` is the older card layout.

## SeganOS

Each of these shows only when it is there, and none of them starts a process per render; the daemon reads them on its own schedule.

- **OS**: `SeganOS <version>` from `/etc/seganos-release`.
- **Kernel**: `Nox <version> (Linux <version>)` when the running kernel is Nox, and the kerneld profile (Balanced, Audio, Game, Performance) from kerneld's socket, or `/var/lib/segan-kerneld/mode`.
- **Compositor**: the Fenestra version from `fenestractl`; on other desktops, `XDG_CURRENT_DESKTOP`.
- **GPU**: name, driver and version, Vulkan API version.
- **Audio**: PipeWire's clock rate and quantum as latency in milliseconds, read with `pw-metadata`.
- **GP Mix**: running or not, and its version.

Elsewhere the logo is a neutral mark; on SeganOS it is the Eclipse.

## Private by default

Census prints nothing about your network unless you ask. Edit `~/.config/census/config.toml`:

```toml
[privacy]
lan_ip = true
public_ip = false   # asks an outside service, once per refresh
tailnet_ip = false
```

## Options

```
census [--layout classic|compact|dashboard] [--logo auto|eclipse|neutral|none]
       [--width N] [--height N] [--color auto|always|never]
       [--json] [--snapshot FILE]
```

`--json` prints the snapshot; `--snapshot` renders a saved one.

Config lives in `~/.config/census/config.toml`: `[layout]` (`style`, `logo`, `sections`), `[privacy]`, `[daemon]` (`socket_path`, `collector_interval_secs`).

## Why a daemon

Fetch tools collect on every run. Census separates collection from rendering: `censusd` keeps a snapshot warm, `census` reads it. Without the daemon, `census` collects directly, prints a one-line hint, and takes a few tens of milliseconds.

```
            Background
        ┌────────────────┐
        │    censusd     │
        └───────┬────────┘
           Unix socket
        ┌───────▼────────┐
        │     census     │
        └────────────────┘
```

## Install

```bash
cargo install --path crates/census
cargo install --path crates/censusd
mkdir -p ~/.config/systemd/user
cp contrib/censusd.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now censusd
```

## Building

```bash
cargo build --release
cargo test
tools/bench.sh            # census (warm daemon) against fastfetch
```

`tools/ansi2png.py` and `tools/shots.sh` make the screenshots; `tools/eclipse_to_ascii.py` made the Eclipse from the SeganOS artwork.

## Roadmap

- `census --card`: a PNG card of your setup for sharing.
- Each distro's own logo.
- Docker and Tailscale lines (their collectors exist but are not wired in yet), battery.

## License

GPL-3.0
