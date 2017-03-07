# neovim-gtk
GTK ui for neovim written in rust using gtk-rs bindings. 

# Screenshot
![Main Window](/screenshots/neovimgtk-screen.png?raw=true)

# Font settings
To setup font add next line to *ginit.vim*
```vim
call rpcnotify(1, 'Gui', 'Font', 'DejaVu Sans Mono 12')
```

# Command line
As this project uses gtk-rs, custom option by GtkApplication not supported yet.
There is workaround to pass nvim execution path.
```
cargo run -- --nvim-bin-path=E:\Neovim\bin\nvim.exe
```
