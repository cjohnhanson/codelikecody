# Voice Anti-Patterns in Documentation

Before/after pairs for voice anti-patterns that show up in documentation.
Each example shows the problem and the fix.

## Corporate blog openings

**Before:**
> This guide walks you through setting up the development environment
> for the first time. By the end, you'll have a fully configured
> workspace ready for development.

**After:**
> The development environment requires three things: Rust nightly, a
> local Postgres instance, and the `clc` binary on your PATH.

The fix isn't just removing "this guide walks you through" — it's
replacing the meta-commentary about the document with the actual content.
Don't describe what the doc will do. Just do it.

**Before:**
> In this tutorial, you'll learn how to configure the caching layer
> for optimal performance in production environments.

**After:**
> The caching layer ships with conservative defaults. For production,
> two settings usually need adjustment: TTL and max entry count.

## Over-explanation

**Before:**
> Configuration is managed through a YAML file. YAML (YAML Ain't Markup
> Language) is a human-readable data serialization format commonly used
> for configuration files. The configuration file is located at
> `.config/app.yml` in your project root directory.

**After:**
> Configuration lives in `.config/app.yml`.

If the audience is developers, they know what YAML is. If the doc's
purpose is to document the config fields, get to the fields.

## Type mixing: reference material in tutorials

**Before** (in a tutorial):
> Before we begin, here's the project structure:
>
> ```
> src/
> ├── handlers/
> │   ├── auth.rs
> │   ├── users.rs
> │   └── mod.rs
> ├── models/
> │   ├── user.rs
> │   ├── session.rs
> │   └── mod.rs
> ├── config.rs
> ├── db.rs
> ├── errors.rs
> └── main.rs
> ```
>
> Each directory serves a specific purpose...

**After** (tutorial links to reference instead):
> The project follows a standard layout — handlers, models, and
> config at the top level. See the [project structure
> reference](./project-structure-reference.md) for the full tree.
>
> For this tutorial, the only file that matters is `src/handlers/auth.rs`.

The tree diagram is reference material. It describes what exists, not
what to do. In a tutorial, it's a detour. In a reference doc, it's
exactly right.

## Type mixing: exhaustive listings in explanations

**Before** (in an explanation doc):
> The lifecycle has five phases:
>
> | Phase | Entry condition | Exit condition | Timeout | Retry |
> |-------|----------------|----------------|---------|-------|
> | Init | Service created | Config loaded | 30s | 3x |
> | ... 25 more rows ... |

**After** (explanation discusses the design, links to reference):
> The lifecycle is built around five phases, each with its own entry
> and exit conditions. The design choice worth understanding is why
> Init and Ready are separate phases — earlier versions combined them,
> which meant a slow config load could block health checks.
>
> For the full phase table, see the [lifecycle
> reference](./lifecycle-reference.md).

Explanations discuss *why*. The phase table is *what*. Keep them apart.
