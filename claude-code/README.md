# claude-code

Protocol types and session management for talking to Claude Code over
piped stdio.

Provides serde types for the Claude Code streaming NDJSON protocol
(`OutputMessage`, `InputMessage`, etc.) and a `Session` struct that
spawns a Claude Code process, wires up stdin/stdout, and exposes
send/recv over a channel.

## Terms of service

This crate shells out to the `claude` CLI binary and parses its
streaming output. It does not perform any OAuth hacks, authentication
bypass, or other circumvention of Anthropic's terms of service. It
uses Claude Code's public CLI interface exactly as documented.

Anyone using this crate (or clc, which depends on it) is expected to
follow both the letter and the spirit of Anthropic's terms of service,
EULA, and any other applicable agreements.
