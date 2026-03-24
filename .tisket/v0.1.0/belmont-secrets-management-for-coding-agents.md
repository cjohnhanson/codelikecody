---
title: "Belmont: secrets management for coding agents"
status: todo
priority: 2
assignee:
labels: [new-crate, architecture]
depends_on: []
created: 2026-02-28T03:42:30Z
updated: "2026-03-24T04:01:38Z"
---

## Problem

Agents running in the codelikecody ecosystem need to use secrets (API keys,
database URLs, tokens) but secret *values* must never enter the agent's context
window — and therefore never get sent to the inference API. Commands that use
secrets run locally under the human's permission model; that's a separate
concern. The unsolved problem is secret values leaking into tool results that
become part of the conversation transcript.

## Name

Belmont. Three references:

1. **Castlevania** — "What is man? A miserable little pile of secrets." Dracula's
   line from Symphony of the Night. The Belmont family hunts him. A secrets tool
   named after the family that deals with secrets.
2. **Shakespeare** — Belmont is Portia's estate in Merchant of Venice. The
   casket scene: suitors choose between gold, silver, and lead without knowing
   what's inside. Acting correctly on something whose contents you can't see.
3. **Chicago** — Belmont Ave, major L stop, runs east-west through Lakeview.
   Fits the Americana street-name vibe alongside the other tools.

Test harness name: `halsted` (another Chicago street), paralleling how `illinois`
is the test harness for `missouri`.

## Design

### Two layers of protection

1. **Primary**: The agent works with secret *names* only. `belmont run -- <cmd>`
   injects values into the subprocess environment and scrubs them from
   stdout/stderr before the output enters context.
2. **Secondary**: PostToolUse hook scans tool output for known secret values and
   warns if a leak is detected. Can't scrub (output is already in context by
   the time PostToolUse fires) but flags it so the agent knows to use
   `belmont run` next time.

### Secret references — `belmont://`

Modeled on 1Password's `op://` URI scheme. `belmont://SECRET_NAME` is the
canonical reference format everywhere the agent encounters secrets:

- **Prime text**: "Available secrets: `belmont://DATABASE_URL`, `belmont://API_KEY`"
- **Scrubbed output**: actual values replaced with `belmont://DATABASE_URL`
- **Agent reasoning**: the agent refers to secrets by their `belmont://` URI

In shell commands, the agent writes `$DATABASE_URL` (since belmont injects via
env vars). The `belmont://` form is the read-side representation — what the
agent sees everywhere a value was or would be. The reference format is consistent
across all surfaces so the agent has one vocabulary for secrets.

### Why output scrubbing works (and why it must happen in `belmont run`)

Claude Code hooks have a critical constraint: PostToolUse hooks can inject
additional context but **cannot modify the tool response**. The tool result is
already in context by the time the hook fires. Post-hoc scrubbing at the hook
level is impossible for the primary defense.

The solution: `belmont run` wraps the command. The Bash tool runs
`belmont run -- some-command`, and belmont captures stdout/stderr from the
subprocess, scrubs secret values (replacing them with `belmont://NAME`
references), and prints the scrubbed version. The Bash tool sees belmont's
output (already scrubbed), not the raw subprocess output. The scrubbing happens
*before* the tool result is captured, not after.

The PostToolUse hook is the secondary defense — it scans for leaks in tool
output from commands that *weren't* run through `belmont run` and warns the
agent. It can't fix the leak, but it can educate.

### Inspiration from 1Password CLI

The `op run` command does similar work: wraps a subprocess, injects secrets from
1Password as env vars, and masks secret values in stdout/stderr by default.
Key difference: `op run` trusts the subprocess (it has the real values in env).
For agents, the subprocess is also trusted (it runs locally under the human's
permissions). The protection is about what enters the context window, not what
the subprocess can do.

### Config format — `belmont.yml`

Lives at project root. Uses vals-style `ref+BACKEND://PATH` URIs to declare
where each secret's value comes from. The config contains references, never
values — safe to commit.

```yaml
backends:
  age:
    identity: ref+keyring://belmont/age-identity

secrets:
  DATABASE_URL: ref+age://secrets.age#/DATABASE_URL
  API_KEY: ref+keyring://belmont/API_KEY
  STRIPE_KEY: ref+age://secrets.age#/STRIPE_KEY
  SIMPLE_THING: ref+env://SIMPLE_THING
```

The `backends` section configures backend-specific settings. Backend configs
can themselves be vals-style references, but only to leaf backends (no cycles).
Resolution order: resolve backend configs first (from leaf backends only), then
resolve secrets using the configured backends. This is a DAG enforced at config
validation time.

### Storage backends (v1)

Three pure-Rust backends for v1:

1. **`ref+env://VAR_NAME`** — reads from process environment via
   `std::env::var()`. Leaf backend, no auth needed.

2. **`ref+keyring://SERVICE/ACCOUNT`** — reads from OS credential store via the
   `keyring` crate (macOS Keychain, Windows Credential Manager, Linux
   secret-service). Leaf backend, authenticated by the OS (biometric, login
   password).

3. **`ref+age://PATH#/KEY`** — decrypts an age-encrypted file via the `age`
   crate (pure Rust, same library behind `rage`), extracts a value by key.
   Requires an identity (private key or passphrase) — configured in the
   `backends` section, typically sourced from keyring. The encrypted file can
   live in the repo since it's useless without the identity.

Credential chaining is a well-established pattern (sops needs an age key, Vault
needs a token, AWS has a whole credential chain). The root of trust is always
something the OS protects (keyring) or the human controls (env). The chain is
short, explicit (declared in config), and acyclic.

### Crate structure

New crate at `belmont/` in the workspace:

```
belmont/
├── Cargo.toml
└── src/
    ├── main.rs          # thin entry point
    ├── lib.rs            # pub mod + re-exports
    ├── cli.rs            # Args, Command, run(), run_command()
    ├── config.rs         # BelmontConfig, BackendConfig, parse/load
    ├── error.rs          # Error enum, Result alias
    ├── registry.rs       # resolve refs → actual values, backend dispatch
    ├── runner.rs          # subprocess spawn, env injection, output capture + scrub
    ├── scrub.rs           # replace secret values with belmont:// references
    └── backend/
        ├── mod.rs         # Backend trait, ref+ URI parsing
        ├── env.rs         # ref+env:// — std::env::var
        ├── keyring.rs     # ref+keyring:// — keyring crate
        └── age.rs         # ref+age:// — age crate
```

Follows the same conventions as tisket and missouri: standalone binary + library
consumed by clc. Uses camino, clap derive, thiserror, serde_yml, portable-pty,
tokio.

### CLI commands

```
belmont init                     # create belmont.yml with empty config
belmont list                     # print declared belmont:// references (never values)
belmont check                    # verify all secrets resolvable, exit 1 if missing
belmont run -- <command>         # inject secrets + scrub output + execute
```

### Key components

**config.rs** — `BelmontConfig { backends: BTreeMap<String, BackendConfig>,
secrets: BTreeMap<String, String> }`. Secrets map name → ref+ URI string.
Load from `belmont.yml` at project root.

**backend/mod.rs** — `Backend` trait with `resolve(&self, uri: &str) -> Result<String>`.
Parse `ref+BACKEND://PATH` URIs, dispatch to the right backend implementation.
Leaf backends (env, keyring) can resolve immediately. Non-leaf backends (age)
require their config to be resolved first from leaf backends.

**registry.rs** — `SecretRegistry` orchestrates resolution: parse config, resolve
backend configs from leaf backends, then resolve all secrets. Exposes:
- `missing()` — names of unresolvable secrets
- `resolved()` — name/value pairs (for scrubber and env injection)

**scrub.rs** — Stateful streaming scrubber. `Scrubber::new(entries)` takes
name/value pairs, sorts values longest-first (so a value that's a substring
of another gets replaced correctly), filters empty values. The scrubber
maintains a trailing boundary buffer of `max_secret_length` bytes between
reads to handle secret values that span chunk boundaries:
- `feed(chunk) -> String` — accepts a chunk of output, returns the
  safely-scrubbed prefix. Retains up to `max_secret_length` bytes as a
  boundary buffer that will be resolved on the next feed or flush.
- `flush() -> String` — emits any remaining buffered bytes at EOF, scrubbing
  as needed.

**runner.rs** — `belmont run` spawns the command inside a PTY via
`portable-pty`. The PTY ensures the subprocess behaves as if connected to a
real terminal (colors, interactive output, buffering behavior). An async read
loop reads from the PTY, feeds chunks through the streaming `Scrubber`, and
writes scrubbed output to real stdout incrementally. Exits with subprocess
exit code. Note: PTY merges stdout and stderr into a single stream — this
matches how the Bash tool captures output and is acceptable for the agent
use case.

### clc integration

**`clc/src/belmont.rs`** — follows the tisket.rs pattern:
- `BelmontState { initialized, secret_count, available_count, missing, secret_names }`
- `detect(project_dir)` → graceful degradation if not initialized
- `ClcTool` impl:
  - `prime()` lists available `belmont://` references + usage rules (never ask
    for values, never read values from files, use `belmont run` for commands
    that need secrets)
  - `status_basic()` one-liner: "belmont: 3/3 secrets available"
  - `status_full()` name-by-name availability
- `check_leak(project_dir, tool_response)` → scans Bash output for secret
  values, returns warning string if found (doesn't reveal which secret)

**`clc/src/hook.rs`** — wire belmont into:
- `assemble_prime()` — add belmont section after missouri
- `assemble_reinforcement()` — add belmont status_basic
- `PostToolUse` handler — call `check_leak` for Bash tool results

**`clc/src/cli.rs`** + **`clc/src/main.rs`** — add `Belmont` subcommand,
`cmd_belmont()` dispatcher.

**Cargo.toml changes** — add `"belmont"` to workspace members, add
`belmont = { path = "../belmont" }` to clc dependencies.

### Implementation order

1. Create belmont crate (cargo init, Cargo.toml, workspace members)
2. error.rs — error enum
3. config.rs — config structs + load/parse + tests
4. backend/ — Backend trait + ref+ URI parsing + env backend
5. backend/keyring.rs — keyring backend
6. backend/age.rs — age backend
7. scrub.rs — scrubber + unit tests
8. registry.rs — resolution orchestration (backend config → secrets)
9. runner.rs — subprocess execution with injection + scrubbing
10. cli.rs — command definitions, run/run_command
11. main.rs + lib.rs — wire up binary
12. clc/src/belmont.rs — detect + ClcTool + check_leak
13. clc/src/hook.rs — integrate into prime, reinforcement, PostToolUse
14. clc/src/cli.rs + clc/src/main.rs — add Belmont subcommand

### Dependencies

- `portable-pty` — cross-platform PTY. Part of wezterm, mature, well-maintained.
  Used by runner.rs to spawn the subprocess inside a pseudoterminal.
- `tokio` — async runtime for the PTY read loop.

### Known limitations for v1

- **No file content scrubbing**: If a command writes a secret to a file and the
  agent reads that file with the Read tool, the secret enters context unscrubbed.
  The PostToolUse check_leak partially addresses this for Bash responses but not
  Read responses. Prime text instructs the agent not to read secret-containing
  files.
- **Three backends only**: env, keyring, age. Future: 1Password, Vault, AWS
  Secrets Manager, sops, file.
- **No signal handling**: Ctrl+C during `belmont run` relies on default
  propagation via PTY. Robust signal handling (like missouri's signal.rs) is a
  follow-up.
