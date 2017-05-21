PREFIX?=/usr/local

test:
	RUST_BACKTRACE=1 cargo test

run:
	RUST_BACKTRACE=1 cargo run

install: install-resources
	cargo install --root $(PREFIX)

install-resources:
	mkdir -p $(PREFIX)/share/applications/
	cp desktop/nvim-gtk.desktop $(PREFIX)/share/applications/
	mkdir -p $(PREFIX)/share/icons/hicolor/128x128/apps/
	cp desktop/nvim-gtk.png $(PREFIX)/share/icons/hicolor/128x128/apps/
	mkdir -p $(PREFIX)/share/fonts/
	cp -n desktop/dejavu_font/*.ttf $(PREFIX)/share/fonts/
	fc-cache -fv
