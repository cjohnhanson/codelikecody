# 🧛 belmont

> _What is a man? A miserable little pile of secrets._ Insecure
> 'best-effort' secret management for supplying credentials to LLM agents.

Belmont resolves secrets from pluggable backends and injects them into
commands at runtime. It scrubs secret values from command output in real
time so agents can use credentials without seeing them.

## Threat model

The goal is to prevent the most common LLM agent exfiltration patterns:
an agent cat'ing a `.env` file, echoing an environment variable while
troubleshooting API auth, or otherwise directly reading credentials
through standard shell operations.

An agent that actively tries to extract secrets through side channels
(inspecting `/proc` for a subshell's environment, running a localhost
echo server and curling to it) can probably succeed. This is a
solo-developed codebase. I am not a security researcher. Do not use
this for anything security-critical. I do not use this in my own
professional work.


## How it works

Declare secrets in `belmont.yml` as backend URIs:

```yaml
secrets:
  DATABASE_URL: "ref+env://DATABASE_URL"
  API_KEY: "ref+keyring://belmont/API_KEY"
```

Run a command with secrets injected:

```
belmont run -- curl -H "Authorization: Bearer $API_KEY" https://api.example.com
```

Belmont resolves each secret from its backend, sets the environment
variables, executes the command, and replaces any secret values in
stdout/stderr with `belmont://NAME` before the output reaches the
agent.

## Backends

- **Environment** (`ref+env://VAR`) — reads from the host environment.
  Read-only.
- **Keyring** (`ref+keyring://SERVICE/ACCOUNT`) — reads from the OS
  credential store (macOS Keychain, Windows Credential Manager, Linux
  secret-service). Supports both read and write.

## Usage

```
belmont init              # create belmont.yml
belmont list              # show declared secret references (never values)
belmont check             # verify all secrets are resolvable
belmont set <name> [val]  # store a secret in its backend
belmont run <command>     # execute with secrets injected, output scrubbed
```
