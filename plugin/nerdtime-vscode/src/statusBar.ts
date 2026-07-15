// SPDX-License-Identifier: AGPL-3.0-only

import * as vscode from 'vscode'
import { Tracker } from './tracker'

export class StatusBarManager {
  private item: vscode.StatusBarItem
  private interval: ReturnType<typeof setInterval> | undefined

  constructor(private tracker: Tracker) {
    this.item = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100)
    this.item.command = 'nerdtime.toggle'
    this.item.tooltip = 'Click to toggle time tracking'
    this.item.show()
  }

  startPolling(): void {
    this.update()
    this.interval = setInterval(() => this.update(), 10000)
  }

  async update(): Promise<void> {
    const status = await this.trackerStatus()
    if (status) {
      this.item.text = `$(watch) nerdtime — ${status.elapsed}`
    } else {
      this.item.text = `$(clock) nerdtime`
    }
  }

  private trackerStatus(): Promise<{ project: string; elapsed: string } | null> {
    return new Promise((resolve) => {
      const { execFile } = require('child_process')
      const config = vscode.workspace.getConfiguration('nerdtime')
      const cliPath = config.get<string>('cliPath') ?? 'nerd'
      execFile(cliPath, ['status', '--json'], { timeout: 3000 }, (err: Error | null, stdout: string) => {
        if (err || !stdout) { resolve(null); return }
        try { resolve(JSON.parse(stdout)) }
        catch { resolve(null) }
      })
    })
  }

  dispose(): void {
    if (this.interval) clearInterval(this.interval)
    this.item.dispose()
  }
}
