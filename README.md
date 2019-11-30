# neovim-gtk [![Build status](https://ci.appveyor.com/api/projects/status/l58o28e13f829llx/branch/master?svg=true)](https://ci.appveyor.com/project/daa84/neovim-gtk/branch/master)[![Build status](https://github.com/daa84/neovim-gtk/workflows/Ubuntu/badge.svg)](https://github.com/daa84/neovim-gtk/actions)
GTK ui for neovim written in rust using gtk-rs bindings. With [ligatures](https://github.com/daa84/neovim-gtk/wiki/Configuration#ligatures) support.

# Screenshot
![Main Window](/screenshots/neovimgtk-screen.png?raw=true)

For more screenshots and description of basic usage see [wiki](https://github.com/daa84/neovim-gtk/wiki/GUI)

# Configuration
To setup font add next line to `ginit.vim`
```vim
call rpcnotify(1, 'Gui', 'Font', 'DejaVu Sans Mono 12')
```
for more details see [wiki](https://github.com/daa84/neovim-gtk/wiki/Configuration)

# Install
## From sources
First check [build prerequisites](#build-prerequisites)

By default to `/usr/local`:
```
make install
```
Or to some custom path:
```
make PREFIX=/some/custom/path install
```

## archlinux
AUR package for neovim-gtk https://aur.archlinux.org/packages/neovim-gtk-git
```shell
git clone https://aur.archlinux.org/neovim-gtk-git.git
cd neovim-gtk-git 
makepkg -si
```
## openSUSE
https://build.opensuse.org/package/show/home:mcepl:neovim/neovim-gtk

## windows
Windows binaries on appveyor
[latest build](https://ci.appveyor.com/api/projects/daa84/neovim-gtk/artifacts/nvim-gtk-mingw64.7z?branch=master)

# Build prerequisites
## Linux
First install the GTK development packages. On Debian/Ubuntu derivatives
this can be done as follows:
```
apt install libatk1.0-dev libcairo2-dev libgdk-pixbuf2.0-dev libglib2.0-dev libgtk-3-dev libpango1.0-dev
```

On Fedora:
```bash
dnf install atk-devel cairo-devel gdk-pixbuf2-devel glib2-devel gtk3-devel pango-devel  
```

Then install the latest rust compiler, best with the
[rustup tool](https://rustup.rs/). The build command:
```
cargo build --release
```

## Windows
Neovim-gtk can be compiled using MSYS2 GTK packages. In this case use 'windows-gnu' rust toolchain.
```
SET PKG_CONFIG_PATH=C:\msys64\mingw64\lib\pkgconfig
cargo build --release
```
