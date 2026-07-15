-- SPDX-License-Identifier: AGPL-3.0-only

local M = {}

local defaults = {
  auto_start = true,
  auto_stop = true,
  cli_path = 'nerd',
  project_detection = 'git',
  statusline = {
    enabled = true,
    format = 'nerdtime — %s',
  },
  notify = true,
  startup_delay = 100,
}

local opts = {}
local active_project = nil

function M.setup(user_opts)
  opts = vim.tbl_deep_extend('force', defaults, user_opts or {})

  vim.api.nvim_create_augroup('nerdtime', { clear = true })

  if opts.auto_start then
    vim.api.nvim_create_autocmd('VimEnter', {
      group = 'nerdtime',
      callback = function()
        vim.defer_fn(function()
          local project = M.detect_project()
          if project then M.start(project) end
        end, opts.startup_delay)
      end,
    })
  end

  if opts.auto_stop then
    vim.api.nvim_create_autocmd('VimLeavePre', {
      group = 'nerdtime',
      callback = function()
        M.stop()
      end,
    })
  end

  vim.api.nvim_create_autocmd('DirChanged', {
    group = 'nerdtime',
    callback = function()
      local project = M.detect_project()
      if project and project ~= active_project then
        if active_project then M.stop() end
        M.start(project)
      end
    end,
  })

  require('nerdtime.commands').setup(opts)
end

function M.detect_project()
  if opts.project_detection == 'git' then
    local ok, result = pcall(vim.fn.system, 'git rev-parse --show-toplevel 2>/dev/null')
    if ok and result and result ~= '' then
      return vim.fn.fnamemodify(vim.trim(result), ':t')
    end
  end
  return vim.fn.fnamemodify(vim.fn.getcwd(), ':t')
end

function M.start(project)
  if not project or project == '' then return end
  local cmd = { opts.cli_path, 'start', project }
  vim.fn.jobstart(cmd, {
    on_exit = function(_, code)
      if code == 0 then
        active_project = project
        if opts.notify then
          vim.notify('nerdtime: tracking ' .. project, vim.log.levels.INFO)
        end
      end
    end,
  })
end

function M.stop()
  if not active_project then return end
  local cmd = { opts.cli_path, 'stop' }
  vim.fn.jobstart(cmd, {
    on_exit = function(_, code)
      if code == 0 then
        if opts.notify then
          vim.notify('nerdtime: tracking stopped', vim.log.levels.INFO)
        end
        active_project = nil
      end
    end,
  })
end

function M.status()
  local cmd = { opts.cli_path, 'status' }
  vim.fn.jobstart(cmd, {
    on_stdout = function(_, data)
      for _, line in ipairs(data) do
        if line and line ~= '' then
          vim.notify(line, vim.log.levels.INFO)
        end
      end
    end,
  })
end

function M.statusline()
  if not active_project then return '' end
  return string.format(opts.statusline.format, active_project)
end

return M
