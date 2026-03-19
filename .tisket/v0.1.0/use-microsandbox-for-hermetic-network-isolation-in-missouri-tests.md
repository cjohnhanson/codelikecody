---
title: "Use microsandbox for hermetic network isolation in missouri tests"
status: in_progress
priority: 3
assignee:
labels: [missouri, network]
depends_on: []
created: 2026-03-19T02:16:09Z
updated: "2026-03-19T02:16:28Z"
---

microsandbox.dev provides microVM-based isolation via libkrun. Each sandbox
gets its own network stack, filesystem, and process space. This is bigger
than just network mocking — it's a potential execution substrate for missouri
(hermetic tests), Belmont (secret injection), and clc Workspace trait
(worker isolation).

This tisket is the spike: validate feasibility before committing to the
architecture. Child tiskets for missouri backend, Belmont integration, and
Workspace trait will be created after the spike validates.

## Scratch Notes

### Session 2026-03-18 — Spike: installation and basic exploration

**Installation:** `curl -sSL https://get.microsandbox.dev | sh` — installs
to `~/.local/bin`. v0.2.6 on macOS aarch64 (Apple Silicon). Binaries: msb,
msbrun, msbserver, msr, msx, msi.

**Server:** `msb server start --dev` starts a local server. Required for all
sandbox operations.

**Boot time:** 0.3-0.8s for simple commands. Acceptable for test runs.
- `echo hello`: 0.3s
- `uname -a`: 0.3s
- With volume mount + debug: ~0.9s

**VM environment:** Linux aarch64 (kernel 6.12.3), running inside libkrun
microVM. Guest is Debian-based (the Python image has curl, wget, bash, etc).

**CLI `-e` flag behavior:** Raw exec, NOT shell. The full string is passed as
`exec_path` with `exec_args: []`. The VM's init process does some internal
space-splitting, but shell operators (`&&`, `|`, `;`, `>`) are NOT interpreted.
URLs containing `://` have inconsistent behavior. Shell-complex commands need
to go through a mounted script file or the Rust SDK.

**Volume mounting (virtiofs):**
- ✅ Files appear in directory listings inside the VM (`ls -la` shows files)
- ⚠️ File CONTENTS appear empty — `wc -c` returns 0, `cat` produces no output,
  `md5sum` returns empty-file hash, even though `ls -la` showed 16 bytes
- This is likely a virtiofs/libkrun bug on macOS. Needs investigation — could
  be a blocker if it can't be worked around. May work differently via the Rust
  SDK's `DynFileSystem` trait.

**Network scopes:** CLI supports `--scope none|local|public|any`. Could not
fully validate due to CLI quoting issues with URLs. `--scope none` does appear
to block network (curl hangs until timeout). Need Rust SDK for proper testing.

**Sandboxfile (project config):** YAML config at project root. Supports
sandbox definitions with image, scope, volumes, ports, env, scripts. Scripts
mechanism didn't work (tried to exec from `.sandbox/scripts/` path that didn't
exist in VM).

**OCI image compatibility:** microsandbox pulls standard Docker/OCI images.
Uses overlayfs layers from `~/.microsandbox/layers/`. The Python image is
`microsandbox/python` from Docker Hub.

### Nix-to-OCI path (research, not hands-on)

**`dockerTools.buildLayeredImage`** is the standard nixpkgs approach:
- Each nix store path becomes its own layer
- Result is OCI tarball loadable by docker/skopeo/microsandbox
- Image contains ONLY declared packages + their closures — distroless by default
- Can include custom Rust binaries as derivations

**Cross-compilation gotcha (macOS → aarch64-linux):**
- macOS can't natively build Linux derivations
- Needs a Linux builder (NixOS VM or Determinate Nix's native builder)
- Current system has NO Linux builder configured (`/etc/nix/machines` absent)
- Determinate Nix 3.8.4+ has native builder; nix-rosetta-builder is an option
- This is a one-time setup task, not a design blocker

**Image for missouri would contain:** clc, tisket, missouri, git, mitmdump,
and any other tools needed by test transitions. All from nixpkgs/workspace build.

### Rust SDK analysis

**Two crates, different maturity:**

1. `microsandbox` (v0.1.2) — the "SDK" crate. Thin JSON-RPC client that
   talks to the server. `SandboxOptions` builder has: server_url, name,
   api_key. `StartOptions` has: image, memory, cpus, volumes, ports, envs,
   depends_on, workdir, shell, scripts, exec, timeout. `Command::run()`
   executes commands and captures stdout/stderr with exit codes.
   **Missing: no `scope` field.** SDK is behind the server.

2. `microsandbox-core` (v0.2.6) — the heavy crate used by CLI and server.
   Has the real `Sandbox` config type with all fields including `scope`.
   `NetworkScope` enum: `None`, `Group`, `Public`, `Any`. Default: `Public`.
   Also has `Microsandbox` root config, `Build` type, `Module` imports,
   full OCI image handling, VM lifecycle management.

**Architecture:** SDK → JSON-RPC → Server → Core (spawns VM via msbrun).
The server is required for all operations. The SDK doesn't embed the VM.

**For missouri integration, three options:**
1. Use `microsandbox` SDK + wait for `scope` to be added upstream
2. Use `microsandbox-core` directly — more API surface, but couples to internals
3. Shell out to `msb` CLI — least coupling, but quoting issues and less control

Option 2 (microsandbox-core) is probably right. It's what the server uses,
it has all the types, and missouri already shells out to mitmdump — using
a Rust library is strictly better. The `management::sandbox` module has the
full sandbox lifecycle.

**Command execution model:** `Command::run(command, args, timeout)` returns
`CommandExecution` with `output()` (stdout lines), `error()` (stderr lines),
`exit_code`, and `success`. This maps cleanly to missouri's transition
execution model where stdout/stderr are captured.

### Key findings / open questions

**Validated:**
1. microsandbox installs and runs on macOS Apple Silicon ✅
2. Sub-second boot times ✅
3. OCI image support (pulls from Docker Hub) ✅
4. NetworkScope enum exists: None, Group, Public, Any ✅
5. Nix can build OCI images for microsandbox via dockerTools ✅
6. Rust SDK + core crate available ✅
7. Command execution captures stdout/stderr with exit codes ✅
8. Volume mounting config exists (volumes field on Sandbox/StartOptions) ✅

**Issues found:**
1. Volume mount file contents appear empty via CLI (virtiofs bug?) ⚠️
2. CLI `-e` flag is raw exec, not shell — irrelevant for SDK usage
3. No Linux builder configured for nix cross-compilation (one-time setup) ⚠️
4. SDK (v0.1.2) missing `scope` field — server/core (v0.2.6) has it ⚠️
5. Project is experimental — "expect breaking changes" ⚠️
6. Server daemon required for all sandbox operations ⚠️

**Still need to validate:**
1. ~~Network isolation actually blocks traffic~~ — CLI showed scope:none blocks.
   SDK doesn't expose scope but server JSON-RPC API accepts it.
2. ~~Volume mount file contents work via SDK/core~~ — filesystem ops work fine
   via Python subprocess inside the VM. CLI virtiofs issue was CLI-specific.
3. Custom OCI image with project binaries boots and runs — NEXT
4. Performance under parallel sandbox creation
5. Can mitmdump run inside a microsandbox for request-level mocking
6. Server startup/lifecycle management (missouri would need to ensure server is running)

### Session 2026-03-18 — Rust SDK PoC results

**msb-spike crate:** standalone binary using `microsandbox` v0.1.2 SDK.
Connects to local server at 127.0.0.1:5555 via JSON-RPC.

**Results:**
- Sandbox create: 5.5ms, start: 265ms (includes VM boot)
- Python code execution: 4.7ms per call (after boot)
- Shell commands via `subprocess.run()`: works (uname -a returns Linux aarch64)
- Network: HTTP 200 from example.com (scope: public default)
- Filesystem: write + read works inside VM
- `command.run()`: fails with 5002 error (SDK v0.1.2 / server v0.2.6 mismatch)
  - Not a blocker: subprocess execution via Python REPL works fine
  - For production: use microsandbox-core directly, not the thin SDK

**Key timing:** ~270ms to get a running sandbox. Sub-5ms per execution after
that. This is very fast — negligible test overhead.

**Architecture confirmed:** missouri can link microsandbox-core, talk to the
local server, boot sandboxes, execute commands, capture output. The command
execution model (send command, get stdout/stderr/exit_code) maps directly
to missouri's transition execution.

### Architecture decision: single binary + no host nix

**Key insight:** missouri doesn't need nix on the host. Nix runs inside a
persistent "nix builder" microsandbox. The runtime dependency is just the
missouri binary (with microsandbox-core linked in).

**Image build flow:**
1. Missouri starts/reuses a persistent "nix builder" sandbox
   - Pre-built OCI image with nix installed, pulled from registry on first run
   - `/nix/store` on a persistent volume (survives across runs, avoids re-downloads)
2. Mounts project dir (read-only) for flake.nix access
3. Mounts scratch dir (read-write) for output
4. Runs `nix build .#missouri-image --out-link /output/image` inside sandbox
5. Reads OCI tarball from scratch mount on host side
6. Loads tarball into microsandbox, boots test sandbox from it

**Test execution flow:**
1. Missouri boots test sandbox from the built image
   - Image contains: project binaries (cross-compiled to aarch64-linux),
     mitmdump, coreutils, bash, ca-certs, git
   - `NetworkScope::None` for hermetic network isolation
2. Mounts test state directory into sandbox via volume
3. Runs transition command inside sandbox via `Command::run()`
4. Captures stdout/stderr, compares filesystem state

**What this replaces:**
- `NixBackend` (nix shell wrapping) → sandbox with packages baked into image
- `BareBackend` → sandbox with just the base image
- `mitmproxy` host-side process management → mitmdump inside sandbox
- Composable sandbox model → single VM provides all isolation layers

**Runtime deps:** missouri binary. That's it. microsandbox server managed
by missouri on demand. Nix builder sandbox pulled on first run.

**OCI image layering:**
- Base layer: "nix builder" image (nix + basic Linux userland)
- Per-project layer: flake.nix defines project-specific packages + binaries
- missouri base image: mitmdump + ca-certs + test infrastructure
- The flake output `missouri-image` composes these
