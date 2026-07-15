# nerdtime Neovim Plugin — Implementation Plan

## Overview

A thin Lua plugin for Neovim that integrates nerdtime's session-based time tracking. Follows the same pattern as WakaTime's vim plugin: the plugin is a thin shell that calls the `nerd` CLI binary via `jobstart()`, keeping all business logic (Git detection, SQLite, sync) in the CLI.

## Architecture

```
┌──────────────────────────┐
│  nerdtime-nvim (Lua)     │
│                          │
│  Autocommands:           │
│  • VimEnter → start      │
│  • VimLeavePre → stop    │
│  • DirChanged → switch   │
│  • BufReadPost → verify  │
│                          │
│  Commands:               │
│  • :NerdStart [project]  │
│  • :NerdStop             │
│  • :NerdStatus           │
│  • :NerdSync             │
│  • :NerdLog              │
│                          │
│  Statusline:              │
│  • require('nerdtime')   │
│    .statusline()         │
│  → "⏱ project — 1h 23m" │
└──────────┬───────────────┘
           │ vim.fn.jobstart() / vim.fn.system()
           ▼
┌──────────────────────────┐
│  nerd CLI                │
│  (already built)         │
│                          │
│  nerd start <project>    │
│  nerd stop               │
│  nerd status --json      │
│  nerd sync               │
│  nerd log --limit 1      │
└──────────────────────────┘
```

## Design decisions

- **Session-based, not heartbeat-based.** nerdtime tracks sessions (start → stop), not continuous heartbeats like WakaTime. The plugin starts a session when you open Neovim and stops when you close it. No 2-minute heartbeat spam.
- **Project auto-detection.** Plugin extracts the project name from the working directory (basename of git root or CWD). Passes it to `nerd start <project>`.
- **Async via `jobstart()`.** All CLI calls are non-blocking. Editor remains responsive.
- **Offline-friendly.** The CLI handles local SQLite storage and sync separately. The plugin never talks to the network.

## Files

```
plugin/nerdtime-nvim/
├── lua/nerdtime/
│   ├── init.lua           # setup(), autocommands, project detection
│   ├── commands.lua       # :NerdStart, :NerdStop, :NerdStatus, etc.
│   └── statusline.lua     # statusline component with elapsed time
├── plugin/
│   └── nerdtime.lua       # entry point: loads immediately, calls setup()
├── doc/
│   └── nerdtime.txt       # help file (vimdoc format)
├── Makefile               # lint (stylua), test (nvim --headless)
├── .stylua.toml           # Lua formatting config
└── README.md              # installation + usage
```

## Plugin details

### `plugin/nerdtime.lua` — Entry point

```lua
-- Loaded on startup. Requires neovim 0.9+ for vim.fn.jobstart.
-- Delegates to lua/nerdtime/init.lua
if vim.fn.has('nvim-0.9') ~= 1 then
  vim.notify('nerdtime requires Neovim 0.9+', vim.log.levels.WARN)
  return
end
require('nerdtime').setup()
```

### `lua/nerdtime/init.lua` — Core module

**`setup(opts)`** — configures the plugin:

```lua
local default_opts = {
  auto_start = true,         -- start tracking on VimEnter
  auto_stop = true,          -- stop tracking on VimLeavePre
  cli_path = 'nerd',         -- path to nerd binary (or just 'nerd' if in PATH)
  project_detection = 'git', -- 'git' | 'cwd' | 'prompt'
  statusline = {
    enabled = true,
    format = '⏱ %s — %s',   -- project — elapsed
  },
  notify = true,             -- show vim.notify on start/stop
  startup_delay = 100,       -- ms to wait after VimEnter before starting
}
```

**Autocommands registered in `setup()`:**

| Autocommand | Event | Action |
|---|---|---|
| `NERD_VimEnter` | `VimEnter` | Wait `startup_delay` ms, detect project, call `nerd start <project>` |
| `NERD_VimLeavePre` | `VimLeavePre` | Call `nerd stop` synchronously (blocking) |
| `NERD_DirChanged` | `DirChanged` | Stop current session, detect new project, start new session |
| `NERD_BufReadPost` | `BufReadPost` | If no active session, detect project and auto-start (configurable) |

**`detect_project()`** — extracts project name:

```lua
function M.detect_project()
  if opts.project_detection == 'git' then
    local ok, git_root = pcall(vim.fn.system, 'git rev-parse --show-toplevel 2>/dev/null')
    if ok and git_root ~= '' then
      return vim.fn.fnamemodify(git_root, ':t')
    end
  end
  -- Fallback: use basename of CWD
  return vim.fn.fnamemodify(vim.fn.getcwd(), ':t')
end
```

**`call_cli(args, opts)`** — shells out to CLI:

```lua
function M.call_cli(args, callback)
  local cmd = vim.fn.extend({ opts.cli_path }, args)
  if async then
    vim.fn.jobstart(cmd, {
      on_exit = function(_, exit_code)
        if exit_code ~= 0 then
          vim.notify('nerdtime: command failed', vim.log.levels.ERROR)
        end
        if callback then callback(exit_code) end
      end,
    })
  else
    local output = vim.fn.system(cmd)
    if callback then callback(vim.v.shell_error, output) end
  end
end
```

### `lua/nerdtime/commands.lua` — User commands

| Command | Implementation |
|---|---|
| `:NerdStart [project]` | `call_cli({'start', project or detect_project()})` |
| `:NerdStop` | `call_cli({'stop'})` |
| `:NerdStatus` | `call_cli({'status'})` — show in notify or echo |
| `:NerdSync` | `call_cli({'sync'})` |
| `:NerdLog` | `call_cli({'log', '--limit', '10'})` — show in quickfix or notify |
| `:NerdToggle` | If active, stop. If inactive, start with detected project. |

### `lua/nerdtime/statusline.lua` — Statusline component

Calls `nerd status --json` periodically (every 10s, cached) and returns formatted string:

| State | Output |
|---|---|
| Active | `"⏱ nerdtime — 1h 23m"` |
| Inactive | `""` (empty — hide when not tracking) |
| No CLI | `""` with a one-time warning |

```lua
function M.statusline()
  local status = cache.get('status')
  if not status then return '' end
  return string.format(opts.statusline.format, status.project, status.elapsed)
end
```

Usage in user's config:
```lua
require('nerdtime').setup()
vim.o.statusline = '%{v:lua.require("nerdtime").statusline()}%=%l,%c'
```

### `doc/nerdtime.txt` — Vimdoc help

Standard vimdoc format documenting all commands, options, and configuration.

## Implementation order (MVP)

| Step | File | Time |
|---|---|---|
| `plugin/nerdtime.lua` entry point | 15 min |
| `lua/nerdtime/init.lua` setup + autocommands | 1 hr |
| `lua/nerdtime/commands.lua` | 30 min |
| `lua/nerdtime/statusline.lua` | 30 min |
| `doc/nerdtime.txt` | 30 min |
| `README.md` | 30 min |
| `Makefile` + `.stylua.toml` | 15 min |
| **Total** | **~3 hrs** |

## Future enhancements

- **Session switch on `DirChanged`** — if user `:cd` to another git repo, stop current session, start new one for the new project. Already described in autocommands above.
- **Floating window** for `:NerdLog` and `:NerdStatus` — show output in a `vim.api.nvim_open_win()` popup instead of notify.
- **Lualine integration** — provide `require('nerdtime').lualine_component()` for lualine.nvim users.
- **Telescope integration** — `:Telescope nerdtime log` to browse sessions with Telescope.
- **Auto-install CLI** — download `nerd` binary if not found (like WakaTime's `install_cli.py`).
