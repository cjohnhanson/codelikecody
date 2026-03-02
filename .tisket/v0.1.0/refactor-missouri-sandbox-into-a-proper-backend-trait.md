---
title: "Refactor missouri sandbox into a proper backend trait"
status: todo
priority:
assignee:
labels: [missouri]
depends_on: []
created: 2026-03-01T13:04:59Z
updated: "2026-03-02T06:00:00Z"
---

Extract the `Sandbox` enum into a trait so new backends (Docker next) don't require touching every match site.

## Current state

`Sandbox` is an enum with two variants (`None` and `Nix`). Every function that runs a command pattern-matches on it to decide how to build the `Command`. This match appears in:

- **executor.rs**: `run_setup_command` (2 matches — shell and non-shell), `run_single_assertion` (1 match delegating to `build_assertion_command_bare`/`_nix`), `execute_transition` (1 match delegating to `build_command_bare`/`_nix`), plus the `build_*_bare` and `build_*_nix` helper pairs
- **compare.rs**: `run_comparator` (1 match)
- **recorder.rs**: `build_recording_command` (1 match with shell/non-shell nested inside)

Every match site does the same thing: the `Nix` arm prepends `nix shell nixpkgs#pkg1 ... --command` to whatever command was going to run. It's always command wrapping with the same nix CLI boilerplate.

## Trait design

```rust
pub trait Backend: std::fmt::Debug + Send + Sync {
    /// Build a Command for a shell command (sh -c "...").
    fn build_shell_command(
        &self,
        command: &str,
        work_dir: &Utf8Path,
        env: &BTreeMap<String, String>,
        path_env: &str,
    ) -> Command;

    /// Build a Command for a direct (non-shell) command.
    fn build_direct_command(
        &self,
        parts: &[&str],
        work_dir: &Utf8Path,
        env: &BTreeMap<String, String>,
        path_env: &str,
    ) -> Command;
}
```

Two methods because the shell vs. non-shell split is in every call site. Each backend implements both.

### BareBackend

Builds `Command::new("sh").arg("-c").arg(command)` for shell, `Command::new(parts[0]).args(&parts[1..])` for direct. Both with `env_clear()` + env + PATH.

### NixBackend

Wraps with `nix shell nixpkgs#pkg1 ... --command sh -c "..."` for shell, `nix shell ... --command parts[0] parts[1..]` for direct. Same env handling.

### Future: DockerBackend

Wraps with `docker run --rm -v work_dir:/work -w /work image sh -c "..."` or similar. The trait makes this additive — no existing code changes.

## Detection

`detect_sandbox` returns `Box<dyn Backend>` instead of `Sandbox` enum. Same logic, different return type:
- `SandboxConfig::None` → `Box::new(BareBackend)`
- `SandboxConfig::Packages(pkgs)` → `Box::new(NixBackend { nix_bin, packages })`
- `MISSOURI_SANDBOX=preinstalled` → `Box::new(BareBackend)`

## Call site changes

Every match site becomes:

```rust
// Before:
match sandbox {
    Sandbox::None => build_command_bare(transition, work_dir, env, &path_env),
    Sandbox::Nix { nix_bin, packages } => build_command_nix(...),
}

// After:
if transition.shell {
    sandbox.build_shell_command(&transition.command, work_dir, env, &path_env)
} else {
    let parts: Vec<&str> = transition.command.split_whitespace().collect();
    sandbox.build_direct_command(&parts, work_dir, env, &path_env)
}
```

The `build_command_bare`, `build_command_nix`, `build_assertion_command_bare`, `build_assertion_command_nix` helper function pairs collapse into the trait methods.

## Recorder

`recorder.rs` currently takes `&Sandbox` and matches internally. Change to `&dyn Backend`. The recorder needs `.spawn()` rather than `run_tracked()`, so the trait methods return a `Command` (not an `Output`) — the caller decides whether to spawn or run_tracked.

## Config

`SandboxConfig` in graph.rs stays as-is — it's the parsed YAML representation. The trait is the runtime representation after detection.

## Files to modify

| File | Change |
|------|--------|
| `missouri/src/executor.rs` | Define `Backend` trait, `BareBackend`, `NixBackend`. Update `detect_sandbox` return type. Replace all match sites. Remove `build_*_bare`/`build_*_nix` helper pairs. |
| `missouri/src/compare.rs` | Replace `&Sandbox` with `&dyn Backend`, remove match in `run_comparator` |
| `missouri/src/recorder.rs` | Replace `&Sandbox` with `&dyn Backend`, remove match in `build_recording_command` |
| `missouri/src/cli.rs` | Update type at detection call sites (`sandbox` becomes `Box<dyn Backend>`) |
| `missouri/src/graph.rs` | No changes — `SandboxConfig` stays as-is |

## Testing

- Existing missouri tests (cargo test + missouri run) should pass unchanged — behavior is identical
- Existing `detect_sandbox_*` tests updated for `Box<dyn Backend>` return type
- No new tests needed — this is a pure refactor with no behavior change
