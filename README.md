# matrixstat

**iStat-style system monitor for the macOS menu bar, in Matrix green.**

One status item shows a bar per CPU core plus `cpu%  mem  SSD%  ↓rx ↑tx`. The dropdown is laid out like
iStat Menus: a header with the headline value, a two-minute history graph, then aligned rows.
Everything is phosphor green (`#00FF41`) in Menlo on a dark menu; hot values turn red.
Rust, about 1 MB, no Dock icon, no network access, no elevated privileges.

```
 ▂▄▁▁▃▂▁▁▁▁  28%  12G  SSD 63%  ↓0.01M ↑0.01M          ← menu bar
┌──────────────────────────────────────────┐
│ CPU                                 28%  │
│ ▁▂▂▃▂▁▁▂▅▃▂▁▁▁▂▂▃▂▁▁▂▃▂▁▁▁▂▂▃▄▂▁▁▁▂▂▁▁▂▃ │  history, newest right
│ ▆▅▃▂▂▁▁▁▁▁                               │  one bar per core
│ Load                    3.06  3.74  4.01 │
│ Per core                               › │
├──────────────────────────────────────────┤
│ PROCESSES                     CPU    MEM │
│ iTerm2                      17.0%  113 MB│
│ …                                        │
├──────────────────────────────────────────┤
│ MEMORY             12.4 GB / 16.0 GB 78% │
│ ▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇▇ │  stacked app / wired / compressed
│ ● App                             4.2 GB │
│ ● Wired                           3.3 GB │
│ ● Compressed                      5.0 GB │
│ ● Cached files                    2.7 GB │
│ ● Free                            223 MB │
│ Swap                      5.6 / 7.0 GB   │
│ Pressure                          Normal │
├──────────────────────────────────────────┤
│ DISKS                                    │
│ Macintosh HD           171 GB available  │
│ 290 GB of 460 GB used     ██████░░░░ 63% │
├──────────────────────────────────────────┤
│ NETWORK         ↓ 0.01 MB/s  ↑ 0.01 MB/s │
│ ▁▂▅▃▁▁▂▁▁▁▂▇▃▁▁▂▁▁▁▁▃▁▁▁▂▁▁▁▁▂▃▁▁▁▂▁▁▂▁ │  download up, upload down
│ en0             ↓ 0.01 MB/s  ↑ 0.01 MB/s │
│ Since boot      ↓   45.9 GB  ↑   11.4 GB │
├──────────────────────────────────────────┤
│ Uptime                        14d 5h 21m │
│ Show in menu bar                       › │
├──────────────────────────────────────────┤
│ Exit the Matrix                       ⌘Q │
└──────────────────────────────────────────┘
```

## Install

Download `matrixstat.app` from [Releases](https://github.com/huytz/matrixstat/releases)
(Apple Silicon and Intel), drop it in `~/Applications`, open it. Or build from source:

```sh
cargo install --git https://github.com/huytz/matrixstat   # bare binary on $PATH
# or
git clone https://github.com/huytz/matrixstat && cd matrixstat
scripts/install.sh --launchd     # ~/Applications/matrixstat.app + start at login
```

Needs macOS 14+ and, to build, Rust 1.95+.

## What it shows

| Section | Rows |
|---|---|
| Menu bar | per-core bars, CPU %, memory used, root disk used %, network ↓↑ rate; each part can be hidden |
| CPU | total %, 2-minute history, per-core strip, load averages, `Per core` submenu with a gauge per core |
| Processes | top 5 by CPU (percent of one core, as in Activity Monitor) with resident memory |
| Memory | used / total %, stacked history, App · Wired · Compressed · Cached files · Free, swap, memory pressure |
| Disks | every user-visible volume: available space, used gauge |
| Network | total ↓/↑ rate in MB/s, mirrored 2-minute history (auto-scaled), per-interface rates, bytes since boot; loopback, VPN `utun*`, AWDL and bridges are skipped so tunnelled traffic is not counted twice |
| System | uptime |

Values at or above 90 % and memory pressure Warning/Critical are red. Refresh is every 2 s.

## Menu bar toggles

`Show in menu bar ›` has a check item for **Core bars**, **CPU %**, **Memory used** and
**Disk used** and **Network ↓↑**. Changes apply at once and are saved to
`~/Library/Application Support/matrixstat/config` (`key=true` lines). With everything unticked
CPU % stays on so the item never goes blank.

If macOS collapses status items behind `»`, the bar is full: ⌘-drag the item to the right, or
untick parts to make it narrower.

## Options

| Environment | Values | Effect |
|---|---|---|
| `MATRIXSTAT_TITLE` | `cpu,mem,disk,net` (default), any subset, `none` | initial text parts; ignored once the config file exists |
| `MATRIXSTAT_DEMO` | `1` | opens the dropdown on the 3rd tick, for screenshots |

`matrixstat --once` prints one snapshot as text and exits (no GUI), handy for scripts and CI.

## How it works

- [`sysinfo`](https://crates.io/crates/sysinfo) for CPU, processes, disks, network, swap;
  Mach `host_statistics64` for the Activity Monitor memory split and
  `kern.memorystatus_vm_pressure_level` for pressure.
- [`tray-icon`](https://crates.io/crates/tray-icon) + [`tao`](https://crates.io/crates/tao)
  for the status item and run loop. Rows are `NSAttributedString`s (Menlo, four green tones);
  graphs are RGBA canvases rendered in Rust, PNG-encoded and mounted as `NSImageView`s through
  `NSMenuItem.view`. The menu is forced to the dark appearance.
- Info rows are disabled menu items: no hover highlight, no click action.

## Limits

- macOS only (AppKit). Tested on macOS 26/27, Apple Silicon.
- Disk "available" includes purgeable space (what Finder shows), so it is larger than `df`.
- No temperatures or fans yet; the history is 60 samples and not persisted.

[CONTRIBUTING](CONTRIBUTING.md) · [SECURITY](SECURITY.md) · [CHANGELOG](CHANGELOG.md) · [MIT](LICENSE)
