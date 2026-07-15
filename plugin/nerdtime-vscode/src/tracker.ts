// SPDX-License-Identifier: AGPL-3.0-only

import * as vscode from 'vscode'
import * as path from 'path'
import { execFile } from 'child_process'

export class Tracker {
  private activeProject: string | null = null

  getConfig() {
    return vscode.workspace.getConfiguration('nerdtime')
  }

  private cliPath(): string {
    return this.getConfig().get<string>('cliPath') ?? 'nerd'
  }

  detectProject(): string | undefined {
    const folders = vscode.workspace.workspaceFolders
    if (!folders?.length) return undefined

    const root = folders[0].uri.fsPath
    const detection = this.getConfig().get<string>('projectDetection')

    if (detection === 'git') {
      try {
        const result = require('child_process').execFileSync('git', [
          'rev-parse', '--show-toplevel',
        ], { cwd: root, encoding: 'utf-8', timeout: 2000 })
        return path.basename(result.trim())
      } catch {
        // fall through to folder name
      }
    }

    return path.basename(root)
  }

  start(): void {
    const project = this.detectProject()
    if (!project) {
      vscode.window.showWarningMessage('nerdtime: could not detect project')
      return
    }

    execFile(this.cliPath(), ['start', project], (error) => {
      if (error) {
        vscode.window.showErrorMessage(`nerdtime: ${error.message}`)
        return
      }
      this.activeProject = project
    })
  }

  stop(): void {
    execFile(this.cliPath(), ['stop'], (error) => {
      if (error) {
        vscode.window.showErrorMessage(`nerdtime: ${error.message}`)
        return
      }
      this.activeProject = null
    })
  }

  toggle(): void {
    if (this.activeProject) {
      this.stop()
    } else {
      this.start()
    }
  }

  showStatus(): void {
    execFile(this.cliPath(), ['status'], { timeout: 3000 }, (error, stdout) => {
      if (error) {
        vscode.window.showErrorMessage(`nerdtime: ${error.message}`)
        return
      }
      vscode.window.showInformationMessage(stdout.trim())
    })
  }

  sync(): void {
    vscode.window.withProgress(
      { location: vscode.ProgressLocation.Notification, title: 'Syncing nerdtime...' },
      () => new Promise<void>((resolve) => {
        execFile(this.cliPath(), ['sync'], { timeout: 10000 }, (error, stdout) => {
          if (error) {
            vscode.window.showErrorMessage(`nerdtime sync: ${error.message}`)
          } else {
            vscode.window.showInformationMessage(stdout.trim())
          }
          resolve()
        })
      }),
    )
  }

  showLog(): void {
    execFile(this.cliPath(), ['log', '--limit', '10'], { timeout: 3000 }, (error, stdout) => {
      if (error) {
        vscode.window.showErrorMessage(`nerdtime: ${error.message}`)
        return
      }
      vscode.window.showInformationMessage(stdout.trim())
    })
  }

  onWindowFocus(): void {
    // Could re-check active session here if needed
  }

  onWorkspaceChange(): void {
    const project = this.detectProject()
    if (project && project !== this.activeProject) {
      if (this.activeProject) this.stop()
      this.start()
    }
  }

  shouldAutoStart(): boolean {
    return this.getConfig().get<boolean>('autoStart', true)
  }

  shouldAutoStop(): boolean {
    return this.getConfig().get<boolean>('autoStop', true)
  }
}
