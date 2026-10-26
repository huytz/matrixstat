# Changelog

## Unreleased

## 0.1.0 — 2026-10-26

First release.

- Menu bar item: one green bar per CPU core plus `cpu%  mem  SSD%`, each part switchable from
  the dropdown (`Show in menu bar`), persisted in `~/Library/Application Support/matrixstat/config`.
- iStat-style dropdown: section headers with headline values, two-minute CPU and stacked memory
  history graphs (`NSImageView` rows rendered in Rust), per-core strip and submenu, top 5
  processes, memory split in Activity Monitor terms with pressure and swap, per-volume disk rows,
  uptime.
- Matrix theme: Menlo, four green tones on a forced-dark menu, red at ≥ 90 % and on memory
  pressure Warning/Critical; disabled info rows so nothing highlights on hover.
- `--once` headless text snapshot; `MATRIXSTAT_TITLE` and `MATRIXSTAT_DEMO` environment options.
- `scripts/bundle.sh` / `scripts/install.sh` for an ad-hoc signed `.app` and a LaunchAgent.
