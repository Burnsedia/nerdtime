-- SPDX-License-Identifier: AGPL-3.0-only

local M = {}

function M.setup(opts)
  local nerdtime = require('nerdtime')

  vim.api.nvim_create_user_command('NerdStart', function(args)
    local project = args.args ~= '' and args.args or nerdtime.detect_project()
    nerdtime.start(project)
  end, { nargs = '?', complete = 'dir' })

  vim.api.nvim_create_user_command('NerdStop', function()
    nerdtime.stop()
  end, {})

  vim.api.nvim_create_user_command('NerdStatus', function()
    nerdtime.status()
  end, {})

  vim.api.nvim_create_user_command('NerdSync', function()
    local cmd = { opts.cli_path, 'sync' }
    vim.fn.jobstart(cmd)
  end, {})

  vim.api.nvim_create_user_command('NerdToggle', function()
    if nerdtime.statusline() ~= '' then
      nerdtime.stop()
    else
      nerdtime.start(nerdtime.detect_project())
    end
  end, {})
end

return M
