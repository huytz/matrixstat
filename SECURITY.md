# Security

## Model

- matrixstat only reads local system information through public APIs: `sysinfo`
  (`host_processor_info`, `proc_pidinfo`, `getfsstat`), Mach `host_statistics64`, and the
  `kern.memorystatus_vm_pressure_level` sysctl. It runs as your user, asks for no entitlements
  or Accessibility access, and never elevates.
- Its only network calls are the weather lookup: `ipinfo.io/json` (location from your public IP,
  skipped when `MATRIXSTAT_LOCATION` is set) and `api.open-meteo.com` (forecast), every 15 minutes,
  over HTTPS. Untick **Weather** and it still fetches in the background but shows nothing; no
  telemetry, no update check.
- The only file it writes is `~/Library/Application Support/matrixstat/config`
  (four `key=true|false` lines for the menu-bar toggles).
- Process names shown in the dropdown come from the kernel and are rendered as text only.
- Release bundles are ad-hoc signed, not notarized; macOS may ask you to confirm the first
  launch. Build from source if you prefer.

## Out of scope

- Hiding your system load or process names from other users of the same session.

## Reporting a vulnerability

Please do not open a public issue. Email the maintainer (see the GitHub profile of
[@huytz](https://github.com/huytz)) with a description and reproduction steps. You should get a
reply within a week.
