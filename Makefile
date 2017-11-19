PREFIX?=/usr/local

test:
	RUST_BACKTRACE=1 cargo test

run:
	RUST_BACKTRACE=1 cargo run

install: install-resources
	cargo install --force --root $(PREFIX)

install-resources:
	mkdir -p $(PREFIX)/share/nvim-gtk/
	cp -r runtime $(PREFIX)/share/nvim-gtk/ 
	mkdir -p $(PREFIX)/share/applications/
	cp desktop/org.daa.NeovimGtk.desktop $(PREFIX)/share/applications/
	sed -i "s|Exec=nvim-gtk|Exec=$(PREFIX)/bin/nvim-gtk|" $(PREFIX)/share/applications/org.daa.NeovimGtk.desktop
	mkdir -p $(PREFIX)/share/icons/hicolor/48x48/apps/
	cp desktop/org.daa.NeovimGtk.png $(PREFIX)/share/icons/hicolor/48x48/apps/
	mkdir -p $(PREFIX)/share/icons/hicolor/scalable/apps/
	cp desktop/org.daa.NeovimGtk.svg $(PREFIX)/share/icons/hicolor/scalable/apps/
	mkdir -p $(PREFIX)/share/fonts/
	cp -n desktop/dejavu_font/*.ttf $(PREFIX)/share/fonts/
	fc-cache -fv
