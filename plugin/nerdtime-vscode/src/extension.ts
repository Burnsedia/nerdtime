// SPDX-License-Identifier: AGPL-3.0-only

import * as vscode from 'vscode'
import { Tracker } from './tracker'
import { StatusBarManager } from './statusBar'

let tracker: Tracker | undefined
let statusBar: StatusBarManager | undefined

export function activate(context: vscode.ExtensionContext) {
  tracker = new Tracker()
  statusBar = new StatusBarManager(tracker)

  context.subscriptions.push(
    vscode.commands.registerCommand('nerdtime.start', () => tracker!.start()),
    vscode.commands.registerCommand('nerdtime.stop', () => tracker!.stop()),
    vscode.commands.registerCommand('nerdtime.toggle', () => tracker!.toggle()),
    vscode.commands.registerCommand('nerdtime.status', () => tracker!.showStatus()),
    vscode.commands.registerCommand('nerdtime.sync', () => tracker!.sync()),
    vscode.commands.registerCommand('nerdtime.log', () => tracker!.showLog()),
  )

  context.subscriptions.push(
    vscode.window.onDidChangeWindowState((e) => {
      if (e.focused) tracker!.onWindowFocus()
    }),
  )

  context.subscriptions.push(
    vscode.workspace.onDidChangeWorkspaceFolders(() => tracker!.onWorkspaceChange()),
  )

  if (tracker.shouldAutoStart()) {
    tracker.start()
  }

  statusBar.startPolling()
}

export function deactivate() {
  statusBar?.dispose()
  if (tracker?.shouldAutoStop()) {
    tracker.stop()
  }
}
