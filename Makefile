PREFIX     ?= $(HOME)/.local
BINDIR      = $(PREFIX)/bin
APPDIR      = $(PREFIX)/share/applications
ICONDIR     = $(PREFIX)/share/icons/hicolor/scalable/apps
HYPRDIR     = $(HOME)/.config/hypr/conf.d
WAYBARDIR   = $(HOME)/.config/waybar/conf.d
MAKODIR     = $(HOME)/.config/mako/conf.d

.PHONY: build install uninstall

build:
	cargo build --release

install: build
	install -Dm755 target/release/arachne $(BINDIR)/arachne
	install -Dm644 install/applications/arachne.desktop $(APPDIR)/arachne.desktop
	install -Dm644 install/applications/icons/arachne.svg $(ICONDIR)/arachne.svg
	install -Dm644 install/hypr/bindings.conf $(HYPRDIR)/arachne.conf
	install -Dm644 install/waybar/arachne.jsonc $(WAYBARDIR)/arachne.jsonc
	install -Dm644 install/waybar/arachne.css $(WAYBARDIR)/arachne.css
	install -Dm644 install/mako/arachne $(MAKODIR)/arachne
	gtk-update-icon-cache -f -t $(PREFIX)/share/icons/hicolor/ 2>/dev/null || true

uninstall:
	rm -f $(BINDIR)/arachne
	rm -f $(APPDIR)/arachne.desktop
	rm -f $(ICONDIR)/arachne.svg
	rm -f $(HYPRDIR)/arachne.conf
	rm -f $(WAYBARDIR)/arachne.jsonc
	rm -f $(WAYBARDIR)/arachne.css
	rm -f $(MAKODIR)/arachne
