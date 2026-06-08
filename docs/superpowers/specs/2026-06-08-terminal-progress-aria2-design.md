# Terminal Progress And Aria2 Design

## Goal

Improve `pv` terminal output and show progress while downloading and installing packages. Apply the same progress behavior to the Windows bootstrap installer script, and support optional aria2 downloads in a Scoop-inspired way without making aria2 a hard dependency.

## Scope

- Improve `pv install <package>` and `pv upgrade [package]` terminal feedback.
- Show download progress for HTTP downloads, local `file://` copies, package install/extract steps, activation, and manifest writing.
- Support optional aria2 for package downloads.
- Improve `scripts/install.ps1` so installing `pv` itself shows download progress and uses `aria2c` when available.
- Keep existing non-install commands machine-readable enough by avoiding decorative output on `list`, `where`, `ls-remote`, and similar query commands.

## Non-Goals

- Do not add a new `pv config` command in this change.
- Do not make aria2 required.
- Do not redesign all CLI output tables.
- Do not change package manifest format.
- Do not commit changes unless the user explicitly asks for git commits.

## Architecture

Add small UI and download boundaries instead of scattering progress code across the engine.

- `src/terminal.rs` owns terminal rendering helpers:
  - download progress bar when a total byte count is known;
  - spinner when total byte count is unknown;
  - status helpers for start/success/skip messages.
- `src/engine/download.rs` owns download backends:
  - built-in reqwest streaming backend;
  - optional aria2 backend selected from config;
  - local file copy path.
- `src/config.rs` owns download configuration with safe defaults.
- `src/engine/mod.rs` emits high-level install phase messages and passes config into downloads.
- `scripts/install.ps1` owns bootstrap progress for release zip and main bucket zip.

This keeps terminal concerns reusable while keeping installation logic in the engine.

## Download Configuration

Add this optional config section:

```toml
[download]
aria2_enabled = false
aria2_split = 5
aria2_max_connection_per_server = 5
aria2_min_split_size = "5M"
```

Defaults preserve existing behavior. If `aria2_enabled = true` but `aria2c` is not found, `pv` prints a short fallback status and uses the built-in downloader.

The initial implementation reads config only. A future `pv config` command can expose these values without changing the data model.

## Aria2 Behavior

When aria2 is enabled and available, `pv` calls:

```text
aria2c --allow-overwrite=true --auto-file-renaming=false --continue=true --dir <cache-dir> --out <file-name> --split <n> --max-connection-per-server <n> --min-split-size <size> <url>
```

The external aria2 process owns its progress display. After download finishes, `pv` still verifies sha256 with existing code.

For bootstrap installation, PowerShell similarly uses `aria2c` first when present. If absent, it falls back to an internal stream copy with `Write-Progress`.

## Terminal Output

Install flow example:

```text
==> Installing ripgrep@14.1.1
Downloading ripgrep.zip  [##########----------]  8.2 MiB/16.4 MiB  2.1 MiB/s  ETA 00:04
Installing archive ...
Activating ripgrep@14.1.1 ...
Installed ripgrep@14.1.1
```

If the version is already installed:

```text
==> Installing ripgrep@14.1.1
Already installed ripgrep@14.1.1
Activating ripgrep@14.1.1 ...
Using ripgrep@14.1.1
```

CLI handlers should avoid printing a duplicate `installed <input>` after `engine.install` because the engine now reports the real resolved version.

## Error Handling

- Download errors keep using `PvError::Download`.
- Aria2 command failure uses `PvError::CommandFailed`.
- Checksum mismatch keeps deleting the bad file and returns the existing checksum error.
- Progress bars and spinners are finished or cleared before returning errors where practical.
- Bootstrap script keeps `$ErrorActionPreference = "Stop"` and throws if aria2 exits non-zero.

## Testing

Use TDD for implementation.

- Config tests cover default download config and TOML round-trip.
- Download tests cover built-in streaming download with hash verification and local file copy.
- A focused aria2 command builder test verifies flags without requiring `aria2c`.
- CLI/engine tests verify user-facing install output contains resolved package/version and no duplicate generic message.
- Install script tests verify `aria2c` and `Write-Progress` support exists, and the local release zip path still installs without network.

## Self-Review

- Placeholder scan: no TODO, TBD, or unresolved requirement remains.
- Scope check: this is one cohesive feature around terminal progress and download backend selection.
- Ambiguity check: `aria2_enabled` default is explicitly false, and bootstrap script opportunistically uses aria2 when found.
- Consistency check: Rust config field names and TOML keys match the planned implementation.
