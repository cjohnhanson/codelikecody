---
title: "moose: animation inspection and control via CDP Animation domain"
status: in_progress
priority: 2
assignee:
labels: [moose, feature]
depends_on: []
created: 2026-03-27T12:39:25Z
updated: "2026-03-27T12:42:22Z"
---

## Problem

Agents evaluating web UIs can't see animations, transitions, or motion.
The accessibility snapshot captures DOM state at one instant — loading
spinners, fade-ins, scroll-triggered reveals, hover transitions, and
interactive feedback are invisible. An agent doing QA or design review
has to guess whether animations work based on before/after snapshots,
missing timing, easing, intermediate states, and whether the animation
even ran.

Agent-browser doesn't support the Animation domain. This is a
differentiator for moose — the first browser automation CLI with
animation-aware agent tooling.

## Design

### CDP Animation domain

The protocol provides everything needed. Already in the codegen:

**Commands:** `enable`, `disable`, `getCurrentTime`, `getPlaybackRate`,
`releaseAnimations`, `resolveAnimation`, `seekAnimations`, `setPaused`,
`setPlaybackRate`, `setTiming`

**Events:** `animationStarted`, `animationCanceled`, `animationCreated`,
`animationUpdated`

**Types:** `Animation` (id, name, pausedState, playState, playbackRate,
startTime, currentTime, type, source), `AnimationEffect` (delay,
endDelay, duration, easing, keyframes), `KeyframesRule`, `KeyframeStyle`

### CLI commands

**Listing:**
```
moose animation list              Currently running animations
moose animation list --all        Including finished/cancelled
moose animation list --json       Machine-readable for missouri tests
```

Output per animation: element selector, animation name, type
(CSSTransition/CSSAnimation/WebAnimation), play state, current time,
duration, playback rate, easing.

**Control:**
```
moose animation pause             Pause all running animations
moose animation pause <id>        Pause specific animation
moose animation resume            Resume all paused animations
moose animation slow <rate>       Set playback rate (0.1 = 10% speed)
moose animation seek <id> <pct>   Jump to percentage of animation
```

**Waiting:**
```
moose wait --animation            Wait for all animations to finish
moose wait --animation <sel>      Wait for animations on element
```

### Agent QA workflow

1. Open page, `moose animation list` — discover what animates on load
2. `moose animation slow 0.1` — slow everything for inspection
3. Take snapshots at intervals — see intermediate states
4. `moose animation pause` — freeze mid-animation
5. `moose snapshot` — inspect frozen state (element positions, opacity, transforms)
6. `moose animation resume` — let it finish
7. `moose snapshot` — verify final state matches expectations

### Missouri test integration

```yaml
assertions:
  - name: "loading spinner exists and animates"
    command: >
      moose open http://localhost:$PORT/
      && moose animation list --json | jq -e '.[] | select(.name == "spin")'

  - name: "fade-in completes within 500ms"
    command: >
      moose wait --animation '#hero'
      && moose is visible '#hero'

  - name: "no janky animations (all use ease or ease-out)"
    command: >
      moose animation list --json
      | jq -e '[.[] | .effect.easing] | all(. == "ease" or . == "ease-out")'
```

The `--json` output becomes a diffable artifact for missouri filesystem
comparison.

### Implementation

New file: `moose/src/native/animation.rs`

- `handle_animation_list` — enables Animation domain, collects
  `animationStarted` events from the event stream, returns structured
  animation data
- `handle_animation_pause` / `handle_animation_resume` — calls
  `Animation.setPaused`
- `handle_animation_slow` — calls `Animation.setPlaybackRate`
- `handle_animation_seek` — calls `Animation.seekAnimations`
- `handle_wait_animation` — polls `Animation.getCurrentTime` until
  all animations complete or timeout

Wire into `actions.rs` command dispatch. Add `animation` to the CLI
help output.

### Skill update

Add an "Animations" section to the moose skill covering:
- When to use animation inspection (QA, design review, visual testing)
- The slow → pause → snapshot → resume workflow
- Missouri test patterns for animations
- The `--json` output format for assertions

## Acceptance Criteria

- [ ] `moose animation list` shows running animations with name, type,
      play state, duration, current time
- [ ] `moose animation list --json` produces parseable JSON
- [ ] `moose animation pause` freezes all animations; `moose snapshot`
      captures the frozen state
- [ ] `moose animation resume` continues paused animations
- [ ] `moose animation slow 0.1` slows playback rate
- [ ] `moose wait --animation` blocks until all animations finish
- [ ] Missouri tests cover the animation commands
- [ ] Moose skill updated with animation section

## Done When

- An agent can discover, inspect, slow, pause, and wait on animations
  through CLI commands
- Animation state is observable in missouri tests via `--json` output
- The moose skill teaches agents when and how to use animation inspection

## Scratch Notes
