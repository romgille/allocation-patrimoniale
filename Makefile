.PHONY: up down test types dev-api dev-web check wasm pages dev-pages

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

# ---- Version statique (GitHub Pages) : moteur Rust en WebAssembly ----
# Prérequis : rustup target add wasm32-unknown-unknown
#             cargo install wasm-bindgen-cli --version <version de la crate wasm-bindgen>
WASM = target/wasm32-unknown-unknown/release/allocation_wasm.wasm

wasm:
	cargo build --release --locked -p allocation-wasm --target wasm32-unknown-unknown
	wasm-bindgen --target web --no-typescript --out-dir web/wasm-pkg $(WASM)

pages: wasm
	cd web && npm ci && npm run build:pages

dev-pages: wasm
	cd web && npm install && npm run dev:pages
