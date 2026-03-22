<!-- metadata
title: "Getting Started with Missouri"
description: "Create your first state graph test suite"
type: tutorial
-->

# Getting Started with Missouri

Missouri tests CLI tools by modeling their behavior as a graph of filesystem states. A transition says "run this command on these files and the result should look like that." This tutorial builds a two-state graph, watches it pass, watches it fail, and adds an assertion — enough to see whether the model fits your use case. For the concepts behind the model, see [What is Missouri?](/missouri/what-is-missouri).

## Install missouri

Build from source:

```
cargo install --path missouri
```

Or, if working within the codelikecody workspace:

```
cargo build -p missouri
```

Verify the binary is available:

```
missouri --version
```

## Initialize a project

Create a directory and initialize it:

```
mkdir my-project && cd my-project
missouri init
```

This creates a `.missouri/` directory containing:

```
.missouri/
  missouri.yml    # project-level config (empty for now)
  bin/            # shared scripts available on PATH during test runs
  ignore          # gitignore-syntax patterns to exclude from comparison
```

The project-level `missouri.yml` can define environment variables, setup commands, and nix packages. For now, the empty default is fine.

## Create two states

A state is a directory with a `.missouri/missouri.yml` file. The files in the directory represent the expected filesystem at that point in time.

Create the first state -- an empty starting point:

```
missouri state add clean
```

This creates `clean/.missouri/missouri.yml` with an empty config (`{}`).

Now create the second state. This one represents what the filesystem should look like *after* a command runs:

```
missouri state add built
```

Add the expected output file to the `built` state:

```
echo "hello" > built/output.txt
```

The directory tree now looks like:

```
my-project/
  .missouri/
    missouri.yml
    bin/
    ignore
  clean/
    .missouri/
      missouri.yml
  built/
    .missouri/
      missouri.yml
    output.txt
```

## Define a transition

Edit `clean/.missouri/missouri.yml` to define how the `clean` state transitions to the `built` state:

```yaml
transitions:
  - name: "create output"
    command: "echo hello > output.txt"
    target: ../built
```

The fields:

- `name` -- optional human-readable label for test output.
- `command` -- the shell command to execute. Runs via `sh -c` by default.
- `target` -- relative path to the expected target state directory.

Missouri copies the source state's files to a temp directory, runs the command there, then compares the result against the target state's files. If the filesystem matches, the transition passes.

## Run the tests

```
missouri run
```

Output will look something like:

```
  PASS  clean -> built (create output)

1 path, 1 passed
```

Missouri discovered two states, found one transition from `clean` to `built`, executed the command, and verified that the resulting filesystem matched the `built` state.

## Validate without running

To check that the graph is well-formed without executing anything:

```
missouri validate
```

```
valid: 2 state(s), 1 transition(s), 1 root(s)
```

To see the test paths that would run:

```
missouri list
```

## Add an assertion

Assertions are commands that verify properties of a state without modifying it. They run against the state's files after any transitions into that state have been validated.

Edit `built/.missouri/missouri.yml`:

```yaml
assertions:
  - name: "output contains hello"
    command: "cat output.txt"
    stdout: "hello\n"
```

The fields:

- `name` -- optional label for test output.
- `command` -- runs in the state's directory.
- `stdout` -- expected exact stdout. The assertion fails if the actual output differs.

Run again:

```
missouri run
```

```
  PASS  clean -> built (create output)
    PASS  output contains hello

1 path, 1 passed
```

The assertion ran after the transition and verified the file contents.

## See a test fail

Change the expected stdout to something wrong. Edit `built/.missouri/missouri.yml`:

```yaml
assertions:
  - name: "output contains hello"
    command: "cat output.txt"
    stdout: "goodbye\n"
```

Run:

```
missouri run
```

```
  FAIL  clean -> built (create output)
    FAIL  output contains hello
      stdout mismatch:
        expected: "goodbye\n"
        actual:   "hello\n"

1 path, 0 passed, 1 failed
```

Missouri shows the exact mismatch. Fix it back to `"hello\n"` and the suite passes again.

Filesystem mismatches work the same way. If the command produces a file that doesn't exist in the target state, or the contents differ, missouri reports the diff.

## Next steps

- Add more states and chain transitions into multi-step paths. Missouri discovers all root-to-leaf paths automatically.
- Use `comparators` on transitions to ignore volatile files or use custom diff commands. See the [CLI reference](/missouri/cli-reference) for the full `missouri.yml` schema.
- Add `env` to states or the project config to inject environment variables.
- Put shared scripts in `.missouri/bin/` -- they're automatically added to PATH during test runs.
- Use `--verbose` (`-v`) for detailed output, or `--keep-temp` to inspect the temp directories missouri creates.
