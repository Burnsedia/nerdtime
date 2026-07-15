.PHONY: build-cli build-api dev-api db-dev test clean

build-cli:
	cargo build --release -p nerd

build-api:
	cargo build --release -p nerdtime-api

dev-api:
	cd nerdtime-api && cargo start

db-dev:
	docker compose -f docker-compose.dev.yml up -d postgres

db-dev-stop:
	docker compose -f docker-compose.dev.yml down

dev-db-and-api:
	docker compose -f docker-compose.dev.yml up -d postgres
	@echo "Waiting for postgres..."
	@sleep 3
	cd nerdtime-api && cargo start

test:
	cargo test --workspace

clean:
	cargo clean

docker-build:
	docker compose build

docker-up:
	docker compose up -d

docker-down:
	docker compose down

help:
	@echo "nerdtime development commands:"
	@echo "  make build-cli       - Build the nerd CLI in release mode"
	@echo "  make build-api       - Build the API backend in release mode"
	@echo "  make dev-api         - Run the API in dev mode (cargo start)"
	@echo "  make db-dev          - Start PostgreSQL for development"
	@echo "  make dev-db-and-api  - Start PostgreSQL and run API"
	@echo "  make test            - Run all tests"
	@echo "  make docker-build    - Build Docker images"
	@echo "  make docker-up       - Start all Docker services"
	@echo "  make docker-down     - Stop all Docker services"
