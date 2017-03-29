PREFIX?=/usr/local

test:
	RUST_BACKTRACE=1 cargo test

run:
	RUST_BACKTRACE=1 cargo run

install:
	cp target/release/nvim-gtk $(PREFIX)/bin/
	cp desktop/nvim-gtk.desktop $(PREFIX)/share/applications/
	cp desktop/nvim-gtk.png $(PREFIX)/share/pixmaps/
	cp -n desktop/dejavu_font/*.ttf $(PREFIX)/share/fonts/
	fc-cache -fv
