---
title: "clc config"
status: done
priority:
assignee:
labels: [feature]
depends_on: [clc-init]
created: 2026-02-23T02:23:25Z
updated: "2026-02-24T14:49:33Z"
---

Configuration system for clc. Controls behavior like attempt counts for status
transitions, which tools are blocked per phase, main branch name, etc.

Two modes:
- Tracked: config in `.clc/config.yml` committed to repo (shared with team)
- Untracked: config in `.clc/config.yml` but `.clc/` is gitignored. For
  untracked mode, project-wide config needs a storage strategy since it
  disappears on fresh clone.

## Missouri tests

State: project-with-config (`.clc/config.yml` with custom settings)
Assertions:
- clc reads config and respects configured values (e.g., custom attempt count)
- Missing config file → sensible defaults
- Invalid config → clear error message
- Config values override defaults (e.g., custom main branch name)
