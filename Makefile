PREFIX ?= $(HOME)/.local
APP_ID = io.github.vlucas14sp.BlueTime
LINGUAS = pt_BR

.PHONY: build install uninstall

build:
	cargo build --release

install: build
	install -Dm755 target/release/blue-time $(PREFIX)/bin/blue-time
	install -Dm644 data/$(APP_ID).desktop $(PREFIX)/share/applications/$(APP_ID).desktop
	install -Dm644 data/$(APP_ID).svg $(PREFIX)/share/icons/hicolor/scalable/apps/$(APP_ID).svg
	sed -i 's|^Exec=blue-time|Exec=$(PREFIX)/bin/blue-time|' $(PREFIX)/share/applications/$(APP_ID).desktop
	for lang in $(LINGUAS); do \
		install -d $(PREFIX)/share/locale/$$lang/LC_MESSAGES; \
		msgfmt po/$$lang.po -o $(PREFIX)/share/locale/$$lang/LC_MESSAGES/blue-time.mo; \
	done
	gtk4-update-icon-cache -q -t $(PREFIX)/share/icons/hicolor 2>/dev/null || true
	update-desktop-database -q $(PREFIX)/share/applications 2>/dev/null || true

uninstall:
	rm -f $(PREFIX)/bin/blue-time
	rm -f $(PREFIX)/share/applications/$(APP_ID).desktop
	rm -f $(PREFIX)/share/icons/hicolor/scalable/apps/$(APP_ID).svg
	for lang in $(LINGUAS); do \
		rm -f $(PREFIX)/share/locale/$$lang/LC_MESSAGES/blue-time.mo; \
	done
