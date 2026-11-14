# Release install for the macOS app.
#
#   make build     release binaries for sqlg-app and sqlg
#   make install   install those binaries and copy sqlg.app into /Applications
#   make clean     remove build output

.PHONY: build install clean

ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))
APP := /Applications/sqlg.app
STAGE := $(ROOT)/target/sqlg.app
RELEASE_APP := $(ROOT)/target/release/sqlg-app
RELEASE_CLI := $(ROOT)/target/release/sqlg

ifeq ($(wildcard $(HOME)/.cargo/bin),)
CARGO_ROOT := $(HOME)/.local
else
CARGO_ROOT := $(HOME)/.cargo
endif

build:
	cargo build --release -p sqlg-app -p sqlg-cli

install: build
	cargo install --path "$(ROOT)/crates/sqlg-app" --root "$(CARGO_ROOT)" --force --locked
	cargo install --path "$(ROOT)/crates/sqlg-cli" --root "$(CARGO_ROOT)" --force --locked
	rm -rf "$(STAGE)"
	mkdir -p "$(STAGE)/Contents/MacOS" "$(STAGE)/Contents/Resources"
	cp "$(ROOT)/packaging/Info.plist" "$(STAGE)/Contents/Info.plist"
	cp "$(ROOT)/packaging/icon.icns" "$(STAGE)/Contents/Resources/icon.icns"
	cp "$(RELEASE_APP)" "$(STAGE)/Contents/MacOS/sqlg-app"
	cp "$(RELEASE_CLI)" "$(STAGE)/Contents/MacOS/sqlg"
	chmod +x "$(STAGE)/Contents/MacOS/sqlg-app" "$(STAGE)/Contents/MacOS/sqlg"
	-killall sqlg-app
	@i=0; \
	while pgrep -x sqlg-app >/dev/null; do \
		i=$$((i+1)); \
		if [ $$i -gt 40 ]; then echo "sqlg-app is still running" >&2; exit 1; fi; \
		sleep 0.25; \
	done
	rm -rf "$(APP)"
	cp -R "$(STAGE)" "$(APP)"
	xattr -dr com.apple.quarantine "$(APP)" || true
	open "$(APP)"

clean:
	cargo clean --manifest-path "$(ROOT)/Cargo.toml"
