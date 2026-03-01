# claude-code

Protocol types and session management for talking to Claude Code over piped stdio.

Provides serde types for the Claude Code streaming JSON protocol (`OutputMessage`, `InputMessage`, etc.) and a `Session` struct that spawns a Claude Code process, wires up stdin/stdout, and exposes send/recv over a channel.
