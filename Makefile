.PHONY: install dev build build-lib build-app test typecheck lint fmt fmt-check check icons clean

install:
	cargo fetch
	cd ./view && pnpm install

dev:
	cd ./view && pnpm exec tauri dev

build: build-lib build-app

build-lib:
	cargo build --release -p systex

build-app:
	cd ./view && pnpm exec tauri build

test:
	cargo test --workspace

typecheck:
	cd ./view && pnpm exec tsc --noEmit

lint:
	cargo clippy --workspace --all-targets -- -D warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

check: fmt-check lint test typecheck

icons:
	cd ./view && pnpm exec tauri icon src-tauri/icons/icon.png -o src-tauri/icons

clean:
	cargo clean
	rm -rf view/dist view/node_modules
