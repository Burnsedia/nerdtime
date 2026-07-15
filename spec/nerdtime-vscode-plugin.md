# nerdtime VS Code Extension — Implementation Plan

## Overview

A VS Code extension that integrates nerdtime's session-based time tracking. Follows the same thin-shell pattern as the Neovim plugin: calls the `nerd` CLI binary via `child_process`, keeping business logic in the CLI.

## Architecture

```
┌─────────────────────────────┐
│  nerdtime-vscode (TS)      │
│                            │
│  Events:                   │
│  • workspace open → start  │
│  • workspace close → stop  │
│  • window focus → verify   │
│                            │
│  Commands:                 │
│  • nerdtime.start          │
│  • nerdtime.stop           │
│  • nerdtime.status         │
│  • nerdtime.sync           │
│  • nerdtime.log            │
│                            │
│  Status bar:               │
│  • "⏱ project — 1h 23m"   │
│  • Click to start/stop     │
└──────────┬─────────────────┘
           │ child_process.execFile()
           ▼
┌─────────────────────────────┐
│  nerd CLI                   │
│  (already built)            │
│                             │
│  nerd start <project>       │
│  nerd stop                  │
│  nerd status --json         │
│  nerd sync                  │
│  nerd log --limit 5         │
└─────────────────────────────┘
```

## Design decisions

- **Session-based, not heartbeat-based.** Same as Neovim plugin — start on workspace open, stop on close. No continuous heartbeats.
- **Project from workspace folder.** Uses the basename of the workspace root folder (or git root if detected).
- **Async via `child_process.execFile()`.** Non-blocking, uses `vscode.window.withProgress` for long operations.
- **Status bar with click action.** Shows active session; click toggles start/stop.
- **Offline-friendly.** CLI handles SQLite locally. Plugin never talks to the network.

## Files

```
plugin/nerdtime-vscode/
├── src/
│   ├── extension.ts         # activate/deactivate, register commands + events
│   ├── tracker.ts           # auto-start/stop logic, project detection
│   └── statusBar.ts         # status bar item with elapsed updates
├── package.json             # extension manifest (contributes.commands, activationEvents)
├── tsconfig.json
├── .vscodeignore            # ignore node_modules, src/ in published vsix
├── esbuild.js               # or .vscodeignore + esbuild for bundling
├── Makefile                 # build, lint, package
├── README.md                # install + usage
└── CHANGELOG.md
```

## Extension details

### `package.json` — Manifest

```jsonc
{
  "name": "nerdtime",
  "displayName": "nerdtime",
  "description": "Terminal-native time tracking for developers",
  "version": "0.1.0",
  "publisher": "nerdtime",
  "license": "AGPL-3.0-only",
  "engines": { "vscode": "^1.85.0" },
  "activationEvents": [
    "onStartupFinished"       // non-blocking activation
  ],
  "contributes": {
    "commands": [
      { "command": "nerdtime.start", "title": "nerdtime: Start Tracking" },
      { "command": "nerdtime.stop", "title": "nerdtime: Stop Tracking" },
      { "command": "nerdtime.toggle", "title": "nerdtime: Toggle Tracking" },
      { "command": "nerdtime.status", "title": "nerdtime: Show Status" },
      { "command": "nerdtime.sync", "title": "nerdtime: Sync to Cloud" },
      { "command": "nerdtime.log", "title": "nerdtime: Show Recent Sessions" }
    ],
    "configuration": {
      "title": "nerdtime",
      "properties": {
        "nerdtime.autoStart": {
          "type": "boolean",
          "default": true,
          "description": "Auto-start tracking when a workspace is opened"
        },
        "nerdtime.autoStop": {
          "type": "boolean",
          "default": true,
          "description": "Auto-stop tracking when the workspace is closed"
        },
        "nerdtime.cliPath": {
          "type": "string",
          "default": "nerd",
          "description": "Path to the nerd CLI binary"
        },
        "nerdtime.projectDetection": {
          "type": "string",
          "enum": ["git", "folder"],
          "default": "git",
          "description": "How to detect the project name"
        }
      }
    }
  }
}
```

### `src/extension.ts` — Entry point

```typescript
import * as vscode from 'vscode'
import { Tracker } from './tracker'
import { StatusBarManager } from './statusBar'

let tracker: Tracker | undefined
let statusBar: StatusBarManager | undefined

export function activate(context: vscode.ExtensionContext) {
  tracker = new Tracker()
  statusBar = new StatusBarManager(tracker)

  // Register commands
  context.subscriptions.push(
    vscode.commands.registerCommand('nerdtime.start', () => tracker.start()),
    vscode.commands.registerCommand('nerdtime.stop', () => tracker.stop()),
    vscode.commands.registerCommand('nerdtime.toggle', () => tracker.toggle()),
    vscode.commands.registerCommand('nerdtime.status', () => tracker.showStatus()),
    vscode.commands.registerCommand('nerdtime.sync', () => tracker.sync()),
    vscode.commands.registerCommand('nerdtime.log', () => tracker.showLog()),
  )

  // Window events
  context.subscriptions.push(
    vscode.window.onDidChangeWindowState((e) => {
      if (e.focused) tracker.onWindowFocus()
    }),
  )

  // Workspace events
  context.subscriptions.push(
    vscode.workspace.onDidChangeWorkspaceFolders(() => tracker.onWorkspaceChange()),
  )

  // Auto-start if configured
  if (tracker.shouldAutoStart()) {
    tracker.start()
  }

  // Status bar refresh interval (every 10 seconds)
  statusBar.startPolling()
}

export function deactivate() {
  statusBar?.dispose()
  if (tracker?.shouldAutoStop()) {
    tracker.stop()
  }
}
```

### `src/tracker.ts` — Core logic

Uses `child_process.execFile` to call the CLI:

```typescript
import { execFile } from 'child_process'
import * as vscode from 'vscode'

export class Tracker {
  private activeProject: string | null = null

  getConfig() {
    return vscode.workspace.getConfiguration('nerdtime')
  }

  detectProject(): string | undefined {
    const workspaceFolders = vscode.workspace.workspaceFolders
    if (!workspaceFolders?.length) return undefined

    const rootPath = workspaceFolders[0].uri.fsPath
    const detection = this.getConfig().get<string>('projectDetection')

    if (detection === 'git') {
      // Shell out to git to find the root
      const result = execFile('git', ['rev-parse', '--show-toplevel'], { cwd: rootPath })
      // ... parse result
    }

    return path.basename(rootPath)
  }

  async start() {
    const project = this.detectProject()
    if (!project) {
      vscode.window.showWarningMessage('nerdtime: Could not detect project name')
      return
    }

    const cliPath = this.getConfig().get<string>('cliPath') ?? 'nerd'
    execFile(cliPath, ['start', project], (error) => {
      if (error) {
        vscode.window.showErrorMessage(`nerdtime: ${error.message}`)
        return
      }
      this.activeProject = project
      vscode.window.setStatusBarMessage(`✓ Tracking ${project}`, 3000)
    })
  }

  async stop(): Promise<void> {
    return new Promise((resolve) => {
      const cliPath = this.getConfig().get<string>('cliPath') ?? 'nerd'
      execFile(cliPath, ['stop'], (error, stdout) => {
        if (error) {
          vscode.window.showErrorMessage(`nerdtime: ${error.message}`)
        }
        this.activeProject = null
        resolve()
      })
    })
  }

  async getStatus(): Promise<{ project: string; elapsed: string } | null> {
    return new Promise((resolve) => {
      const cliPath = this.getConfig().get<string>('cliPath') ?? 'nerd'
      execFile(cliPath, ['status', '--json'], { timeout: 3000 }, (error, stdout) => {
        if (error || !stdout) { resolve(null); return }
        try { resolve(JSON.parse(stdout)) }
        catch { resolve(null) }
      })
    })
  }

  toggle() {
    if (this.activeProject) { this.stop() }
    else { this.start() }
  }

  shouldAutoStart() { return this.getConfig().get<boolean>('autoStart', true) }
  shouldAutoStop() { return this.getConfig().get<boolean>('autoStop', true) }
}
```

### `src/statusBar.ts` — Status bar widget

```typescript
import * as vscode from 'vscode'
import { Tracker } from './tracker'

export class StatusBarManager {
  private item: vscode.StatusBarItem
  private interval: NodeJS.Timeout | undefined

  constructor(private tracker: Tracker) {
    this.item = vscode.window.createStatusBarItem(
      vscode.StatusBarAlignment.Left,
      100
    )
    this.item.command = 'nerdtime.toggle'
    this.item.show()
  }

  startPolling() {
    this.update()
    this.interval = setInterval(() => this.update(), 10000)
  }

  async update() {
    const status = await this.tracker.getStatus()
    if (status) {
      this.item.text = `$(watch) nerdtime — ${status.elapsed}`
      this.item.tooltip = `Tracking: ${status.project}\nClick to stop`
      this.item.backgroundColor = undefined
    } else {
      this.item.text = `$(clock) nerdtime`
      this.item.tooltip = 'No active session — click to start'
    }
  }

  dispose() {
    clearInterval(this.interval)
    this.item.dispose()
  }
}
```

### `esbuild.js` — Bundler config

Minimal esbuild config to bundle the extension:

```javascript
const esbuild = require('esbuild')
esbuild.build({
  entryPoints: ['src/extension.ts'],
  bundle: true,
  outfile: 'out/extension.js',
  external: ['vscode'],
  format: 'cjs',
  platform: 'node',
  sourcemap: true,
  minify: true,
}).catch(() => process.exit(1))
```

## Distribution

- **Development**: in monorepo at `plugin/nerdtime-vscode/`
- **CI Build**: `vsce package` in CI → produces `.vsix`
- **Marketplace**: publish to VS Code Marketplace when going public

## Implementation order (MVP)

| Step | File | Time |
|---|---|---|
| `package.json` manifest | 15 min |
| `src/tracker.ts` CLI wrapper + project detection | 1 hr |
| `src/statusBar.ts` status bar widget | 30 min |
| `src/extension.ts` activation + commands + events | 45 min |
| `esbuild.js` bundler config | 15 min |
| `README.md` + `CHANGELOG.md` | 30 min |
| `Makefile` (build, lint, package targets) | 15 min |
| **Total** | **~3.5 hrs** |

## Future enhancements

- **Session switching on workspace change** — when user opens a different folder, stop current session, start new one.
- **Output channel** — dedicated `nerdtime` output channel for debug logs.
- **Tree view** — show recent sessions, stats, sync status in VS Code sidebar.
