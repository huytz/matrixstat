# Contributing

Thanks for taking a look. matrixstat is small on purpose; please keep changes focused.

## Development

```sh
cargo build
cargo test                                   # unit tests (formatting, graphs, config, volume filter)
cargo run --release -- --once                # headless snapshot, no GUI
MATRIXSTAT_DEMO=1 cargo run --release        # opens the dropdown after ~6 s for screenshots
cargo fmt --all && cargo clippy --all-targets -- -D warnings
```

`scripts/install.sh` builds and installs `~/Applications/matrixstat.app`; `scripts/bundle.sh`
wraps any built binary into an ad-hoc signed `.app`. Set `MATRIXSTAT_TITLE` or edit
`~/Library/Application Support/matrixstat/config` to test the menu-bar toggles without clicking.

Module map: `stats.rs` sampling → `Snapshot`; `mac.rs` Mach memory split; `render.rs` text and
pixel graphs; `theme.rs` palette and AppKit glue; `menu.rs` dropdown layout; `config.rs` toggles;
`main.rs` run loop.

## Pull requests

- One change per PR, with a short description of the behaviour before and after.
- UI changes: attach a screenshot of the dropdown (`MATRIXSTAT_DEMO=1` + `screencapture -x`).
- CI must be green (fmt, clippy with `-D warnings`, tests, `--once` smoke, bundle) on macOS.
- Keep comments to one line; put longer reasoning in the PR description.

## Reporting bugs

Open an issue with: matrixstat version, macOS version and chip, the output of
`matrixstat --once`, and what the menu bar or dropdown showed instead.
