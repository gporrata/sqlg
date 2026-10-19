# Release install for the macOS app.
#
#   make build     pnpm tauri build
#   make install   quit sqlg, copy the bundle into /Applications, open it

.PHONY: build install

ROOT := $(abspath $(dir $(lastword $(MAKEFILE_LIST))))
BUNDLE := $(ROOT)/target/release/bundle/macos/sqlg.app
APP := /Applications/sqlg.app

build:
	pnpm tauri build

install:
	@if [ ! -d "$(BUNDLE)" ]; then $(MAKE) build; fi
	@test -d "$(BUNDLE)" || { echo "missing $(BUNDLE)" >&2; exit 1; }
	-killall sqlg-app
	@i=0; \
	while pgrep -x sqlg-app >/dev/null; do \
		i=$$((i+1)); \
		if [ $$i -gt 40 ]; then echo "sqlg-app is still running" >&2; exit 1; fi; \
		sleep 0.25; \
	done
	rm -rf "$(APP)"
	cp -R "$(BUNDLE)" "$(APP)"
	xattr -dr com.apple.quarantine "$(APP)"
	cmp "$(BUNDLE)/Contents/MacOS/sqlg-app" "$(APP)/Contents/MacOS/sqlg-app"
	open "$(APP)"
