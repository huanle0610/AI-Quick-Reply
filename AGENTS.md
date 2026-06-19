# AGENTS.md

## Local Build Notes

This project is a Windows Tauri 2 + Angular + Bun desktop app. When the user asks for a final runnable exe, avoid detours and use the shortest verified path below.

### Build the final exe

Prefer this when the user wants a runnable `.exe` rather than an installer:

```powershell
cd C:\RustroverProjects\click-complete\src-tauri
cargo test --release
cargo build --release
```

The final exe is:

```text
C:\RustroverProjects\click-complete\src-tauri\target\release\ai-quick-reply.exe
```

If Angular/frontend files changed, build the frontend first from the repository root:

```powershell
cd C:\RustroverProjects\click-complete
bun run build
```

Then run the `cargo build --release` command from `src-tauri`.

### Full Tauri packaging

Only use this when the user specifically wants installer/bundle output:

```powershell
cd C:\RustroverProjects\click-complete
bun run tauri build
```

If the build reaches a line like this, the runnable exe was already produced even if MSI bundling fails later:

```text
Built application at: C:\RustroverProjects\click-complete\src-tauri\target\release\ai-quick-reply.exe
```

A later WiX `light.exe` failure means MSI packaging failed; it does not mean the release exe failed to compile.

### Dev run

Run dev mode from the repository root:

```powershell
cd C:\RustroverProjects\click-complete
bun run tauri dev
```

Do not launch Tauri dev from another working directory with `--cwd`; Tauri's `beforeDevCommand` may then run in the wrong directory and fail with:

```text
error: Script not found "start"
```

If a one-line explicit PowerShell command is needed, use:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "Set-Location 'C:\RustroverProjects\click-complete'; & 'C:\Program Files\nodejs\node_modules\bun\bin\bun.exe' run tauri dev"
```

### Windows/Bun process notes

On this machine, `bun` resolves to a PowerShell shim at `C:\Program Files\nodejs\bun.ps1`. `Start-Process -FilePath 'bun'` can fail with `%1 is not a valid Win32 application`. If a background process must be started directly, use the real executable:

```text
C:\Program Files\nodejs\node_modules\bun\bin\bun.exe
```

### Common build blockers

- `failed to open ... .cargo-build-lock` / `拒绝访问`: close any running `ai-quick-reply.exe`, `cargo`, or `rustc` process and retry. In the managed sandbox, `cargo build --release` may need escalation because `cargo test` is the only pre-approved Cargo prefix.
- `There is not enough space on the disk`: C: is likely low on free space. Rust debug artifacts are large. Prefer `cargo test --release` for verification. If cleanup is necessary, ask before deleting broad build caches; `src-tauri\target\debug` is reproducible but can be large.
- WiX/MSI bundling failure after the release exe is built: give the user the exe path and mention that only installer packaging failed.

### Verification before handing the exe to the user

After building, confirm the final exe exists:

```powershell
Get-Item C:\RustroverProjects\click-complete\src-tauri\target\release\ai-quick-reply.exe | Select-Object FullName,Length,LastWriteTime
```

Report the absolute path to the user.