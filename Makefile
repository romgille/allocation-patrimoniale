.PHONY: up down test types dev-api dev-web check

up:
	docker compose up -d --build

down:
	docker compose down

types:
	cargo test -q -p engine

test: types
	cargo test -q

dev-api:
	BIND_ADDR=127.0.0.1:8080 cargo run -p api

dev-web:
	cd web && npm install && npm run dev

check: test
	cargo clippy --all-targets -- -D warnings
	cd web && npm ci && npm run typecheck
