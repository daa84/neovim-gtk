PREFIX?=/usr/local

test:
	RUST_BACKTRACE=1 cargo test

run:
	RUST_BACKTRACE=1 cargo run

install:
	mkdir -p $(PREFIX)/bin/
	cp target/release/nvim-gtk $(PREFIX)/bin/
	xdg-desktop-menu install --novendor ./desktop/nvim-gtk.desktop
	xdg-desktop-icon install --novendor ./desktop/nvim-gtk.png
	mkdir -p $(PREFIX)/share/fonts/
	cp -n desktop/dejavu_font/*.ttf $(PREFIX)/share/fonts/
	fc-cache -fv
