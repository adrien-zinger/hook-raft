.PHONY: test
test: cargo_build_debug test/hook
	cd test && sudo ./run.sh

test/hook: target/debug/hook
	cp target/debug/hook test/hook

target/debug/hook: cargo_build_debug

.PHONY: cargo_build_debug
cargo_build_debug:
	cargo build
