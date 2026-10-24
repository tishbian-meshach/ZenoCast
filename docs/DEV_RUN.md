# Running Zenocast Locally During Development

Zenocast provides a fast, local dev loop that builds and runs the application directly from the target directory with no installer required. Tracing logs are printed directly to the console and saved to rotating log files under `target/dev-logs/`.

## The One Command

To build and run Zenocast in debug mode with live logs:

```powershell
.\scripts\dev-run.ps1
```

## Options

- **Release mode** (for performance testing / profiling without installer overhead):
  ```powershell
  .\scripts\dev-run.ps1 -Release
  ```

- **Clean build** (blows away `target/` first for a fresh compilation):
  ```powershell
  .\scripts\dev-run.ps1 -Clean
  ```

- **Clean + Release**:
  ```powershell
  .\scripts\dev-run.ps1 -Clean -Release
  ```

## Auto-Rebuild Watcher

For continuous development, launch the auto-rebuilder which polls for Rust source changes and relaunches the app:

```powershell
.\scripts\dev-watch.ps1
```

## Developer Overrides (`dev.toml`)

You can create a `dev.toml` file in the project root (git-ignored) by copying `dev.toml.example`:

```powershell
Copy-Item dev.toml.example dev.toml
```

Available options:
- `skip_license_check = "true"`: Bypasses license enforcement during local development.
- `mock_sources = "true"`: Uses synthetic video/camera patterns instead of physical hardware.
- `default_project = "path/to/project.arxproj"`: Automatically opens the specified project on startup.
