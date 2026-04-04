# moose

> Predominantly a browser, the moose's diet consists of both terrestrial
> and aquatic vegetation, depending on the season, with branches, twigs
> and dead wood making up a large portion of their winter diet. —Wikipedia

Browser automation for AI agents. Forked from Vercel Labs'
[agent-browser](https://github.com/vercel-labs/agent-browser).

Moose connects to Chrome via the Chrome DevTools Protocol. It can also
target [Lightpanda](https://lightpanda.io/) (a headless browser
engine) or native mobile apps via WebDriver/Appium.

## How it works

Moose runs as a CLI. In daemon mode, it keeps a browser session alive
between commands over a Unix socket, so multiple operations can share
a single browser tab without reconnecting.

## Usage

```
moose navigate <url>       # open a page
moose screenshot            # capture current state
moose click <selector>      # interact with elements
moose type <selector> <text>
moose --help                # full command list
```
