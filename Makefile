# redis-benchmark-rs — shared build/test/release tasks.
# Used by both local dev and CI (.github/workflows/*) so the logic lives in one place.

CARGO       ?= cargo
MUSL_TARGET := x86_64-unknown-linux-musl
BIN         := redis-benchmark-rs
DIST        ?=

.PHONY: all ci fmt fmt-check lint test build musl-tools build-musl rpm deb clean help

all: build

help: ## list targets
	@grep -hE '^[a-z-]+:.*##' $(MAKEFILE_LIST) | sed -E 's/:.*## /\t/' | sort

## ---- checks (what CI runs) ----
ci: fmt-check lint test build-musl ## run the full CI suite

fmt: ## auto-format
	$(CARGO) fmt --all

fmt-check: ## check formatting
	$(CARGO) fmt --all -- --check

lint: ## clippy with warnings as errors
	$(CARGO) clippy --all-targets --all-features -- -D warnings

test: ## unit + integration tests (integration needs redis at REDIS_HOST:REDIS_PORT)
	$(CARGO) test --all --all-features

## ---- builds ----
build: ## release build (gnu, mimalloc)
	$(CARGO) build --release

musl-tools:
	@command -v $(MUSL_TARGET)-gcc >/dev/null 2>&1 || { sudo apt-get update -qq && sudo apt-get install -y musl-tools; }

build-musl: musl-tools ## portable static musl binary -> redis-benchmark-rs-x86_64-musl
	rustup target add $(MUSL_TARGET)
	$(CARGO) build --release --target $(MUSL_TARGET) --no-default-features
	cp target/$(MUSL_TARGET)/release/$(BIN) $(BIN)-x86_64-musl

## ---- packaging (set DIST=el8|el9|jammy|noble to tag the artifact) ----
rpm: ## build an rpm into out/
	$(CARGO) install --locked cargo-generate-rpm
	$(CARGO) build --release
	$(CARGO) generate-rpm
	@mkdir -p out
	@for f in target/generate-rpm/*.rpm; do \
		base=$$(basename "$$f" .rpm); \
		if [ -n "$(DIST)" ]; then cp "$$f" "out/$$base.$(DIST).rpm"; else cp "$$f" "out/$$base.rpm"; fi; \
	done

deb: ## build a deb into out/
	$(CARGO) install --locked cargo-deb
	$(CARGO) build --release
	$(CARGO) deb
	@mkdir -p out
	@for f in target/debian/*.deb; do \
		base=$$(basename "$$f" .deb); \
		if [ -n "$(DIST)" ]; then cp "$$f" "out/$$base.$(DIST).deb"; else cp "$$f" "out/$$base.deb"; fi; \
	done

clean:
	$(CARGO) clean
	rm -rf out $(BIN)-x86_64-musl
