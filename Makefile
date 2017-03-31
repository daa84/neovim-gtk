PREFIX?=/usr/local

test:
	RUST_BACKTRACE=1 cargo test

run:
	RUST_BACKTRACE=1 cargo run

install:
	mkdir -p $(PREFIX)/bin/
	cp target/release/nvim-gtk $(PREFIX)/bin/
	mkdir -p $(PREFIX)/share/applications/
	cp desktop/nvim-gtk.desktop $(PREFIX)/share/applications/
	mkdir -p $(PREFIX)/share/pixmaps/
	cp desktop/nvim-gtk.png $(PREFIX)/share/pixmaps/
	mkdir -p $(PREFIX)/share/fonts/
	cp -n desktop/dejavu_font/*.ttf $(PREFIX)/share/fonts/
	fc-cache -fv
