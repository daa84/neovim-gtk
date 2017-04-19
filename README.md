# neovim-gtk
GTK ui for neovim written in rust using gtk-rs bindings. 

# Screenshot
![Main Window](/screenshots/neovimgtk-screen.png?raw=true)

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
* enable external popup menu autocompletion menu (this function a bit limited, so disabled by default)
```
cargo run -- --enable-external-popup
```

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
