PREFIX ?= /usr/local
BINDIR = $(PREFIX)/bin
DATADIR = $(PREFIX)/share
SYSCONFDIR ?= /etc

.PHONY: build install uninstall

build:
	cargo build --release

install:
	install -Dm755 target/release/hordora $(DESTDIR)$(BINDIR)/hordora
	install -Dm755 resources/hordora-session $(DESTDIR)$(BINDIR)/hordora-session
	install -Dm644 resources/hordora.desktop $(DESTDIR)$(DATADIR)/wayland-sessions/hordora.desktop
	install -Dm644 resources/hordora-portals.conf $(DESTDIR)$(DATADIR)/xdg-desktop-portal/hordora-portals.conf
	install -Dm644 config.example.toml $(DESTDIR)$(SYSCONFDIR)/hordora/config.toml
	for f in extras/wallpapers/*.glsl; do \
		install -Dm644 "$$f" "$(DESTDIR)$(DATADIR)/hordora/wallpapers/$$(basename $$f)"; \
	done

uninstall:
	rm -f $(DESTDIR)$(BINDIR)/hordora
	rm -f $(DESTDIR)$(BINDIR)/hordora-session
	rm -f $(DESTDIR)$(DATADIR)/wayland-sessions/hordora.desktop
	rm -f $(DESTDIR)$(DATADIR)/xdg-desktop-portal/hordora-portals.conf
	rm -rf $(DESTDIR)$(DATADIR)/hordora
	rm -rf $(DESTDIR)$(SYSCONFDIR)/hordora
