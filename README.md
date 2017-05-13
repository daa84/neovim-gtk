# neovim-gtk [![Build status](https://ci.appveyor.com/api/projects/status/l58o28e13f829llx/branch/master?svg=true)](https://ci.appveyor.com/project/daa84/neovim-gtk/branch/master)
GTK ui for neovim written in rust using gtk-rs bindings.

# Screenshot
![Main Window](/screenshots/neovimgtk-screen.png?raw=true)
For more screenshots and description of basic usage see [wiki](https://github.com/daa84/neovim-gtk/wiki/GUI)

# Font settings
By default gnome settings are used:
```bash
gsettings get org.gnome.desktop.interface monospace-font-name
```
To setup font add next line to *ginit.vim*
```vim
call rpcnotify(1, 'Gui', 'Font', 'DejaVu Sans Mono 12')
```

# Command line
* pass nvim custom execution path (by default used `nvim` command)
```
cargo run -- --nvim-bin-path=E:\Neovim\bin\nvim.exe
```
* enable external autocompletion popup menu (this function limited a bit, so disabled by default)
```
cargo run -- --enable-external-popup
```
# Install
## Ubuntu snap package
Not usable for now due to some limitation!

This package also includes neovim, so neovim not needed and if present in system - not used. Install command:
```
sudo snap install nvim-gtk --channel=candidate
```
There is some limitation for package: only `/home` directory available for editing and '~' is mapped to snap home directory.
Config files must be placed in `~/snap/nvim-gtk/common/config/nvim/[g]init.vim` directory

Run command: `nvim-gtk <file_name>` or from dash: `NeovimGtk`.

To run neovim provided by snap package execute: `nvim-gtk.neovim`.

# Build
## Linux
Install GTK development packages. Install latest rust compiler, better use *rustup* tool. Build command:
```
cargo build --release
```

## Windows
Neovim-gtk can be compiled using MSYS2 GTK packages. In this case use 'windows-gnu' rust toolchain.
```
SET PKG_CONFIG_PATH=C:\msys64\mingw64\lib\pkgconfig
cargo build --release
```
