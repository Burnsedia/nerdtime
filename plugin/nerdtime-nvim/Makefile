.PHONY: lint test

lint:
	stylua --check lua/

test:
	nvim --headless -c "lua require('nerdtime').setup()" -c "q"
