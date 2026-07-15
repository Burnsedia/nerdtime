-- SPDX-License-Identifier: AGPL-3.0-only

if vim.fn.has('nvim-0.9') ~= 1 then
  vim.notify('nerdtime requires Neovim 0.9+', vim.log.levels.WARN)
  return
end

require('nerdtime').setup()
