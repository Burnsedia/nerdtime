.PHONY: node_modules compile lint package

node_modules:
	npm install

compile: node_modules
	npm run compile

lint: node_modules
	npm run lint

package: compile
	npm run package
