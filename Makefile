.PHONY: dev build test clean run

dev:
	cargo build

build:
	RUSTFLAGS="-C target-cpu=native" cargo build --release

test:
	cargo test

clean:
	cargo clean
	rm -f tracker.wal

run:
	cargo run