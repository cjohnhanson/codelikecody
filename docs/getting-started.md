<!-- metadata
title: "Getting Started"
description: "Set up clc on a project and complete your first task"
type: tutorial
-->

# Getting Started

## Prerequisites

- Rust stable toolchain
- Git
- [Claude Code](https://docs.anthropic.com/en/docs/claude-code) installed and authenticated

## Install from source

Clone the repository and build the workspace:

```sh
git clone https://github.com/codelikecody/codelikecody.git
cd codelikecody
cargo build --workspace
```

This produces three binaries in `target/debug/`: `clc`, `tisket`, and `missouri`. Add them to your PATH or use full paths in the commands below.

```sh
export PATH="$PWD/target/debug:$PATH"
```

Verify the install:

```sh
clc --help
tisket --help
```

## Initialize a project

Start with an existing git repository. The repo needs to be on the `main` branch (or whichever branch you configure as trunk).

```sh
cd /path/to/your-project
```

### Set up tisket (issue tracker)

```sh
tisket init
```

This creates `tisket.yml` at the repo root and a `.tisket/default/` directory for issues. Commit these:

```sh
git add tisket.yml .tisket/
git commit -m "tisket: init"
```

### Set up clc (workflow engine)

```sh
clc init
```

This creates:

- `.clc/` — state directory for clc
- `.claude/settings.local.json` — Claude Code hook configuration pointing at `clc hook`

The hooks are what make clc work. Every Claude Code event (session start, tool use, stop) passes through `clc hook`, which injects context and enforces constraints.

If you don't want clc files tracked by git, use `--untracked` instead — it writes exclusion patterns to `.git/info/exclude`:

```sh
clc init --untracked
```

Check that everything is wired up:

```sh
clc status
```

You should see `initialized: true`, your branch name, and `is_main: true`.

## Create a tisket project

Tisket organizes issues into projects. The `default` project was created by `tisket init`. For this walkthrough, create a named project:

```sh
tisket project create v1
```

## Create an issue

Create an issue that describes a unit of work:

```sh
tisket issue create -p v1 "Add greeting function"
```

This writes a markdown file under `.tisket/v1/` with YAML frontmatter. The issue ID is a random 4-character prefix plus a slug of the title — something like `ab12-add-greeting-function`. The exact prefix will differ on your machine.

List issues to see it:

```sh
tisket issue list
```

The issue starts in `todo` status by default. Commit the new issue to trunk:

```sh
git add .tisket/
git commit -m "tisket: add greeting function issue"
```

## Pick up the issue

From the main branch, pick up the issue by its ID:

```sh
clc pickup <issue-id>
```

This does several things:

1. Verifies the issue is in a pickable status (`todo`, `blocked`, or `paused`)
2. Sets the issue status to `in_progress` and commits that change on trunk
3. Creates a git worktree at `.worktrees/<issue-id>/`
4. Initializes clc in the worktree (hooks, state directory)
5. Sets the phase to `tests-unwritten`

Change into the worktree to start working:

```sh
cd .worktrees/<issue-id>/
```

Check the state:

```sh
clc status
```

You should see `phase: tests-unwritten` and `is_worktree: true`.

## The phase system

Phases enforce a test-driven workflow. They advance forward one step at a time:

```
tests-unwritten → tests-written → red → implementing → green → review-requested → in-review → reviewed → done
```

Hooks block actions that violate the current phase — you can't write implementation files during `tests-unwritten`, and you can't stop the session before reaching `review-requested`. For the full phase semantics, see [What is codelikecody?](what-is-codelikecody.md).

### Write the test

With the phase at `tests-unwritten`, write a failing test for the greeting function. The specifics depend on your project's language and test framework. As an example in a Rust project:

```rust
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

### See it fail (red)

Run the test suite. The test should fail — the function doesn't exist yet.

Advance to red:

```sh
clc status set red
```

The `red` phase means "tests exist and fail." This is the expected state before implementation.

### Implement

Advance to the implementing phase:

```sh
clc status set implementing
```

Now write the implementation — the minimum code to make the test pass:

```rust
pub fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}
```

### See it pass (green)

Run the test suite again. The test should pass.

Advance to green:

```sh
clc status set green
```

### Commit your work

Commit frequently throughout the process — commits are checkpoints, not milestones. Before finalizing, make sure everything is committed:

```sh
git add src/lib.rs src/tests.rs  # or whatever files you changed
git commit -m "feat: add greeting function"
```

## Complete the work

The review phases (`review-requested`, `in-review`, `reviewed`) sit between `green` and `done`. For this walkthrough, advance through them:

```sh
clc status set review-requested
clc status set in-review
clc status set reviewed
clc status set done
```

Now finalize:

```sh
clc done
```

This closes the tisket issue (setting its status to `done` and moving it to `.tisket/v1/.closed/`) and commits that change.

## Merge back to trunk

Switch back to the main branch:

```sh
cd /path/to/your-project
```

Merge the completed branch:

```sh
clc merge <issue-id>
```

This fast-forward merges the feature branch into trunk, removes the worktree, and deletes the branch. The issue's closed status is now on trunk.

## What's next

- [clc CLI Reference](clc/cli-reference.md) — full command and flag documentation
- [tisket CLI Reference](tisket/cli-reference.md) — issue tracker commands and schema
- [What is codelikecody?](what-is-codelikecody.md) — the reasoning behind the phase system and trunk-protection model
