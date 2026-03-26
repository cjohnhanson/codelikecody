---
name: moose
description: >-
  Browser automation CLI for AI agents — navigating pages, filling forms,
  clicking buttons, taking screenshots, recording screencasts, extracting
  data, testing web apps, or automating any browser task. Moose talks CDP
  directly to Chrome with a persistent daemon for session reuse. Use when
  programmatic web interaction is needed.
user-invocable: false
---

# Browser Automation with Moose

Moose is a browser automation CLI that talks Chrome DevTools Protocol
directly. It runs a persistent daemon that keeps the browser alive across
commands — each `moose` invocation is fast IPC to an already-running
session, not a cold browser launch.

## Core Workflow

Every browser automation follows this pattern:

1. **Navigate**: `moose open <url>`
2. **Snapshot**: `moose snapshot -i` (get element refs like `@e1`, `@e2`)
3. **Interact**: Use refs to click, fill, select
4. **Re-snapshot**: After navigation or DOM changes, get fresh refs

```bash
moose open https://example.com/form
moose snapshot -i
# Output shows refs: textbox "Email" [ref=e1], textbox "Password" [ref=e2], button "Submit" [ref=e3]

moose fill @e1 "user@example.com"
moose fill @e2 "password123"
moose click @e3
moose wait --load networkidle
moose snapshot -i  # Check result
```

## Command Chaining

Commands can be chained with `&&`. The daemon keeps the browser alive
between commands.

```bash
moose open https://example.com && moose wait --load networkidle && moose snapshot -i
moose fill @e1 "user@example.com" && moose fill @e2 "pass" && moose click @e3
```

Chain when you don't need intermediate output. Run separately when you
need to parse snapshot refs before interacting.

## Ref Lifecycle

Refs (`@e1`, `@e2`) are invalidated when the page changes. Always
re-snapshot after:

- Clicking links or buttons that navigate
- Form submissions
- Dynamic content loading (dropdowns, modals, AJAX)

```bash
moose click @e5              # Navigates to new page
moose snapshot -i            # MUST re-snapshot — old refs are dead
moose click @e1              # Use new refs
```

## Recording Screencasts

Moose records browser sessions as WebM video via CDP's screencast API.

### Basic recording

```bash
moose open https://example.com
moose record start /path/to/output.webm
# ... interact with the page ...
moose record stop
```

Recording captures the page render at the browser level. The video
shows exactly what a user would see, minus the cursor (CDP screencast
doesn't capture OS-level input).

### Recording workflow for demos

The typical demo recording flow:

```bash
# Set up viewport for consistent framing
moose set viewport 1280 720

# Open the starting page
moose open https://example.com

# Start recording AFTER navigation so the first frame is the loaded page
moose record start demo.webm

# Pause between actions so viewers can follow
moose highlight '#login-btn' && sleep 1
moose click '#login-btn' && sleep 1

moose snapshot -i  # Get refs for the new page
moose highlight @e1 && sleep 0.5
moose fill @e1 "user@example.com" && sleep 0.5
moose highlight @e2 && sleep 0.5
moose fill @e2 "password" && sleep 0.5
moose highlight @e3 && sleep 0.5
moose click @e3

moose wait --load networkidle && sleep 1

# Stop recording
moose record stop
moose close
```

### Tips for watchable recordings

- **Set viewport first.** `moose set viewport 1280 720` gives a
  consistent frame size.
- **Start recording after the page loads.** Otherwise the first
  seconds are a blank loading screen.
- **Use `highlight` before clicks.** It briefly outlines the target
  element so viewers can see what's about to be clicked.
- **Add `sleep` between actions.** 0.5-1 second pauses give the
  recording time to capture each state. Without them, actions blur
  together.
- **Use `--annotate` for screenshots, not video.** The annotated
  overlay flashes for a single frame in video — useless. Take
  annotated screenshots separately for documentation stills.

### Recording + screenshots combo

For documentation that needs both video and labeled stills:

```bash
moose open https://example.com
moose set viewport 1280 720
moose record start walkthrough.webm

# Take annotated screenshot (labels appear in the PNG, not the video)
moose screenshot step-1-home.png --annotate
sleep 1

moose click @e3 && sleep 1
moose screenshot step-2-projects.png --annotate
sleep 1

moose click @e4 && sleep 1
moose screenshot step-3-blog.png --annotate

moose record stop
moose close
```

This produces a video for the walkthrough and labeled screenshots
for inline documentation.

## Essential Commands

```bash
# Navigation
moose open <url>                 Navigate to URL
moose close                      Close browser
moose back                       Go back
moose forward                    Go forward
moose reload                     Reload page

# Snapshot
moose snapshot -i                Interactive elements with refs
moose snapshot -i -C             Include cursor-interactive elements
moose snapshot -s "#selector"    Scope to CSS selector
moose snapshot -c                Compact (remove empty structural nodes)

# Interaction (use @refs from snapshot)
moose click @e1                  Click element
moose fill @e2 "text"            Clear and type text
moose type @e2 "text"            Type without clearing
moose select @e1 "option"        Select dropdown option
moose check @e1                  Check checkbox
moose uncheck @e1                Uncheck checkbox
moose press Enter                Press key
moose keyboard type "text"       Type at current focus
moose scroll down 500            Scroll page
moose hover @e1                  Hover element
moose highlight @e1              Highlight element (brief visual outline)

# Get information
moose get text @e1               Get element text
moose get html @e1               Get element HTML
moose get value @e1              Get input value
moose get url                    Get current URL
moose get title                  Get page title
moose get count "nav a"          Count matching elements
moose get cdp-url                Get CDP WebSocket URL

# Check state
moose is visible @e1             Check if element is visible
moose is enabled @e1             Check if element is enabled
moose is checked @e1             Check if checkbox is checked

# Wait
moose wait @e1                   Wait for element to appear
moose wait --load networkidle    Wait for network idle
moose wait --url "**/page"       Wait for URL pattern
moose wait --text "Welcome"      Wait for text to appear
moose wait 2000                  Wait milliseconds

# Capture
moose screenshot                 Screenshot to temp dir
moose screenshot --full          Full page screenshot
moose screenshot --annotate      Annotated with numbered labels
moose pdf output.pdf             Save as PDF

# Recording
moose record start <path>        Start WebM recording
moose record stop                Stop and save recording

# Debug
moose console [--clear]          View console logs
moose errors [--clear]           View page errors
moose highlight <sel>            Highlight element visually
moose inspect                    Open Chrome DevTools

# Network
moose network requests           Inspect tracked requests
moose network route "**" --abort Block matching requests
moose network har start          Start HAR recording
moose network har stop file.har  Stop and save HAR

# Viewport & device
moose set viewport 1280 720     Set viewport size
moose set viewport 1920 1080 2  2x retina
moose set device "iPhone 14"    Emulate device

# Tabs
moose tab new                    Open new tab
moose tab list                   List tabs
moose tab close                  Close current tab
moose tab 2                      Switch to tab 2

# Clipboard
moose clipboard read             Read clipboard
moose clipboard write "text"     Write to clipboard

# Sessions
moose --session name open <url>  Named session (isolated)
moose session list               List active sessions
moose close                      Close current session
```

## JavaScript Evaluation

```bash
# Simple expressions
moose eval 'document.title'
moose eval 'document.querySelectorAll("img").length'

# Complex JS: use --stdin to avoid shell quoting issues
moose eval --stdin <<'EVALEOF'
JSON.stringify(
  Array.from(document.querySelectorAll("a"))
    .map(a => ({ text: a.textContent.trim(), href: a.href }))
)
EVALEOF
```

Use `--stdin` for anything with nested quotes, arrow functions, or
template literals. Shell quoting will corrupt complex JS.

## Authentication

**Persistent profile (simplest):**
```bash
moose --profile ~/.myapp open https://app.example.com/login
# ... login once ...
# All future runs: already authenticated
moose --profile ~/.myapp open https://app.example.com/dashboard
```

**Session name (auto-save/restore cookies + localStorage):**
```bash
moose --session-name myapp open https://app.example.com/login
# ... login ...
moose close  # State auto-saved
# Next time: state auto-restored
moose --session-name myapp open https://app.example.com/dashboard
```

**Auth vault (encrypted credentials):**
```bash
echo "$PASSWORD" | moose auth save myapp --url https://app.example.com/login --username user --password-stdin
moose auth login myapp
```

**Connect to user's browser (already logged in):**
```bash
moose --auto-connect state save ./auth.json
moose --state ./auth.json open https://app.example.com/dashboard
```

## Network Interception

```bash
# Block specific requests
moose network route "**/analytics/*" --abort
moose network route "**/ads/*" --abort

# Mock API responses
moose network route "**/api/user" --body '{"name":"Test User"}'

# Record HTTP traffic as HAR
moose network har start
# ... interact ...
moose network har stop ./traffic.har

# Inspect requests made so far
moose network requests --filter "*api*"
```

## Diffing (Verifying Changes)

```bash
# Snapshot -> action -> diff to see what changed
moose snapshot -i
moose click @e2
moose diff snapshot  # Shows accessibility tree diff

# Visual regression
moose screenshot baseline.png
# ... changes made ...
moose diff screenshot --baseline baseline.png

# Compare two URLs
moose diff url https://staging.example.com https://prod.example.com
```

## Common Patterns

### Form submission
```bash
moose open https://example.com/signup
moose snapshot -i
moose fill @e1 "Jane Doe"
moose fill @e2 "jane@example.com"
moose select @e3 "California"
moose check @e4
moose click @e5
moose wait --load networkidle
```

### Data extraction
```bash
moose open https://example.com/products
moose get text body > page.txt
moose snapshot -i --json  # Machine-readable
```

### Responsive testing
```bash
moose set viewport 1920 1080 && moose screenshot desktop.png
moose set viewport 375 812 && moose screenshot mobile.png
moose set device "iPhone 14" && moose screenshot device.png
```

### Working with iframes
```bash
moose snapshot -i
# Iframe refs are inline — interact directly
moose fill @e3 "4111111111111111"  # Card field inside iframe
```

## Configuration

Create `moose.json` in the project root for persistent settings:

```json
{
  "headed": true,
  "proxy": "http://localhost:8080",
  "profile": "./browser-data"
}
```

Priority: `~/.moose/config.json` < `./moose.json` < env vars < CLI flags.

## Environment Variables

Key variables (all prefixed `MOOSE_`):

| Variable | Purpose |
|---|---|
| `MOOSE_SESSION` | Session name (default: "default") |
| `MOOSE_HEADED` | Show browser window |
| `MOOSE_EXECUTABLE_PATH` | Custom Chrome path |
| `MOOSE_COLOR_SCHEME` | dark, light, no-preference |
| `MOOSE_ALLOWED_DOMAINS` | Restrict navigation domains |
| `MOOSE_DEFAULT_TIMEOUT` | Action timeout in ms (default: 25000) |
| `MOOSE_IDLE_TIMEOUT_MS` | Auto-shutdown daemon after inactivity |
| `MOOSE_SCREENSHOT_DIR` | Default screenshot directory |
| `MOOSE_CONTENT_BOUNDARIES` | Wrap output in boundary markers |
| `MOOSE_MAX_OUTPUT` | Truncate output to N chars |

## Security

All security features are opt-in:

- **Domain allowlist**: `MOOSE_ALLOWED_DOMAINS="example.com,*.example.com"`
- **Action policy**: `MOOSE_ACTION_POLICY=./policy.json`
- **Content boundaries**: `MOOSE_CONTENT_BOUNDARIES=1` wraps page content in markers
- **Output limits**: `MOOSE_MAX_OUTPUT=50000` prevents context flooding
