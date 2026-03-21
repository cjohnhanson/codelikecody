<!-- metadata
title: "Getting Started"
description: "Set up clc on a project and complete your first task"
type: tutorial
-->

# Getting Started

By the end of this, you'll have clc managing a real project: an agent that can't edit source files until tests exist, can't stop until work is done, and produces reviewable branches that merge cleanly.

## Prerequisites

- Rust stable toolchain
- Git
- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) installed and authenticated

## Install from source

```sh
git clone https://github.com/codelikecody/codelikecody.git
cd codelikecody
cargo build --workspace
```

This produces three binaries in `target/debug/`: `clc`, `tisket`, and `missouri`. Add them to your PATH:

```sh
export PATH="$PWD/target/debug:$PATH"
```

Verify:

```sh
clc --help
tisket --help
```

Both should print their help text. If they don't, the build didn't produce the binaries — check `cargo build` output for errors.

## Initialize a project

Start in an existing git repository on its main branch.

```sh
cd /path/to/your-project
```

### Set up tisket

```sh
tisket init
```

This creates `tisket.yml` at the repo root and a `.tisket/default/` directory. Commit them — tisket's state lives in git, and clc expects it to be committed before branching.

```sh
git add tisket.yml .tisket/
git commit -m "tisket: init"
```

### Set up clc

```sh
clc init
```

This does two things:

1. Creates `.clc/` — clc's state directory (phase tracking, worker state)
2. Creates `.claude/settings.local.json` — hooks that wire every Claude Code event through `clc hook`

The hooks are what make everything work. Without them, clc is just a CLI. With them, every tool call the agent makes passes through clc's guard before it executes.

Check the wiring:

```sh
clc status
```

You should see output like:

```
initialized: true
untracked: false
main_branch: main
branch: main
is_main: true
is_worktree: false
```

If `initialized: false`, the `.clc/` directory wasn't created. Run `clc init` again.

If you want clc invisible to version control (no `.clc/` or `.claude/` in the repo), use `--untracked`:

```sh
clc init --untracked
```

This writes exclusion patterns to `.git/info/exclude` instead.

## Create work

Tisket organizes issues into projects. `tisket init` created a `default` project. For this walkthrough, create a project to group your work under:

```sh
tisket project create myproject
```

Now create an issue:

```sh
tisket issue create -p myproject "Add greeting function"
```

Tisket generates a short random prefix and slugifies the title. The output is the issue ID — something like `ab12-add-greeting-function`. Your prefix will be different.

```sh
tisket issue list
```

```
ID                         STATUS  TITLE
ab12-add-greeting-function todo    Add greeting function
```

The issue starts at `todo` — ready for an agent (or you) to pick up. Commit it to trunk:

```sh
git add .tisket/
git commit -m "tisket: add greeting function issue"
```

## Pick up the issue

This is where clc takes over. From the main branch:

```sh
clc pickup <issue-id>
```

Replace `<issue-id>` with your actual ID (e.g., `ab12-add-greeting-function`, or just `ab12` — tisket resolves short prefixes).

What happens:

1. clc checks that the issue is pickable (`todo`, `blocked`, or `paused`)
2. The issue status changes to `in_progress` and that change gets committed on trunk
3. A git worktree appears at `.worktrees/<issue-id>/` with a new branch
4. clc initializes itself inside the worktree — hooks, state directory, phase set to `tests-unwritten`

Move into the worktree:

```sh
cd .worktrees/<issue-id>/
```

Check where you are:

```sh
clc status
```

```
initialized: true
untracked: false
main_branch: main
phase: tests-unwritten
branch: ab12-add-greeting-function
is_main: false
is_worktree: true
tisket: ab12-add-greeting-function (in_progress) — 1 open
```

The phase is `tests-unwritten`. This means:

- **You can** read any file, run read-only commands, edit files under `tests/missouri/`
- **You cannot** edit source files, and the hooks will block any attempt with an explanation
- **You cannot** stop the session — the stop hook rejects until you reach `review-requested`

## Write a failing test

The phase system exists to enforce one thing: tests come before implementation. Right now, source file edits are blocked. Write the test first.

The specifics depend on your project. In a Rust project, you might write:

```rust
// tests/greeting_test.rs (or wherever your tests live)
#[test]
fn greeting_includes_name() {
    let result = greet("world");
    assert_eq!(result, "Hello, world!");
}
```

Once the test file exists, advance the phase:

```sh
clc status set tests-written
```

No output means success. Check with `clc status` — the phase should now say `tests-written`.

## Watch it fail

Run your test suite. The test should fail — the `greet` function doesn't exist yet. This is the "red" in red-green-refactor: a test that fails for the right reason.

```sh
cargo test  # or your project's test command
```

The test fails. Good. Advance to red:

```sh
clc status set red
```

This confirms that you've verified the test actually fails. Without this step, you could write a test that passes trivially and skip straight to implementation — the phase system won't let you.

## Implement

Now advance to `implementing`:

```sh
clc status set implementing
```

This is the phase where everything unlocks. All file edits are allowed. Write the minimum code to make the test pass:

```rust
pub fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}
```

## Get green

Run the test suite again:

```sh
cargo test
```

The test passes. Advance to green:

```sh
clc status set green
```

At `green`, source file edits lock again. Only test files are editable. This is intentional — if you need to change implementation after getting green, go back to `implementing` first (backward phase transitions are always allowed).

## Commit

Commit frequently throughout the process — don't wait until the end. But before finalizing, make sure everything is committed:

```sh
git add src/lib.rs tests/greeting_test.rs
git commit -m "feat: add greeting function"
```

## Review and finalize

Between `green` and `done`, three review phases handle handoff between the agent and a reviewer. They won't matter in this walkthrough — there's no reviewer — but they're worth seeing in passing because they're where the [phase system](/clc/phase-system)'s permission locking gets interesting in real use.

Advance through them:

```sh
clc status set review-requested
clc status set in-review
clc status set reviewed
clc status set done
```

In a real workflow, `review-requested` is the first point where an agent is *allowed to stop* — everything before that, the stop hook rejects. During `in-review`, edits unlock so review feedback can be addressed. After `reviewed`, edits lock again. The [phase system docs](/clc/phase-system) cover the full permission matrix.

Now finalize:

```sh
clc done
```

This closes the tisket issue (status → `done`, file moved to `.tisket/myproject/.closed/`) and commits that change. The working tree must be clean — uncommitted changes cause `clc done` to fail.

## Merge to trunk

Go back to the main branch:

```sh
cd /path/to/your-project
```

Merge the completed work:

```sh
clc merge <issue-id>
```

This fast-forward merges the feature branch into trunk, deletes the branch, and removes the worktree. The closed tisket is now on trunk. The work is done.

## What you just did

You created an issue, picked it up, wrote a test before implementation (because the hooks wouldn't let you do it the other way), implemented the feature, and merged a clean branch. The phase system enforced the workflow mechanically — not through prompts or instructions, but through tool-call interception.

In practice, an agent does this autonomously. The agent receives a tisket, enters the worktree, and the hook system constrains its behavior at each phase. The agent doesn't need to know the rules — the guard enforces them.

## Next

- [The Phase System](/clc/phase-system) — deeper explanation of phases, restrictions, and the guard
- [Multi-Agent Orchestration](/clc/orchestration) — dispatch multiple agents to work in parallel
- [clc CLI Reference](/clc/cli-reference) — every command and flag
- [tisket CLI Reference](/tisket/cli-reference) — issue management commands and schema
