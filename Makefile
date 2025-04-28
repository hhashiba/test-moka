ifeq ($(shell uname -s), Darwin)
	CPU_CORES = $(shell sysctl -n hw.ncpu)
else
	CPU_CORES = $(shell grep -c processor /proc/cpuinfo)
endif

ARG = --help

.PHONY:	help
help: ## show help message.
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

.PHONY: check
check: ## check compile is succeed
	@cargo check -j $(CPU_CORES)

.PHONY:	build
build: ## build application
	@cargo test -j $(CPU_CORES) --no-run --locked

.PHONY: update_cargo
update_cargo: ## build application
	@cargo test -j $(CPU_CORES) --no-run

.PHONY:	run
run: ## run binary: e.g. make run ARG=--help && make run ARG=-V
	@make build
	@cargo build -j $(CPU_CORES) --locked
	@./target/debug/test-moka ${ARG}

.PHONY:	test
test: ## run: cargo test
	@cargo test

.PHONY: test_debug
test_debug: ## run: cargo test -- --nocapture (print debug mode)
	@cargo test -- --nocapture

.PHONY:	clean
clean: ## run: cargo clean
	@cargo clean

.PHONY: format
format: ## run: cargo clippy && cargo fmt
	@./script/cargo_format.sh
