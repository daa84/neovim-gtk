" A Neovim plugin that implements GUI helper commands
if !has('nvim') || exists('g:GuiLoaded')
	finish
endif
let g:GuiLoaded = 1

if exists('g:GuiInternalClipboard')
	let s:LastRegType = 'v'
	function! provider#clipboard#Call(method, args) abort
		if a:method == 'get'
			return [rpcrequest(1, 'Gui', 'Clipboard', 'Get', a:args[0]), s:LastRegType]
		elseif a:method == 'set'
			let s:LastRegType = a:args[1]
			call rpcnotify(1, 'Gui', 'Clipboard', 'Set', a:args[2], join(a:args[0], ''))
		endif
	endfunction
endif

" Set GUI font
function! GuiFont(fname, ...) abort
	call rpcnotify(1, 'Gui', 'Font', s:NvimQtToPangoFont(a:fname))
endfunction

" Some subset of parse command from neovim-qt
" to support interoperability
function s:NvimQtToPangoFont(fname)
	let l:attrs = split(a:fname, ':')
	let l:size = -1
	for part in l:attrs
		if len(part) >= 2 && part[0] == 'h'
			let l:size = strpart(part, 1)
		endif
	endfor

	if l:size > 0
		return l:attrs[0] . ' ' . l:size
	endif

	return l:attrs[0]
endf


" The GuiFont command. For compatibility there is also Guifont
function s:GuiFontCommand(fname, bang) abort
	if a:fname ==# ''
		if exists('g:GuiFont')
			echo g:GuiFont
		else
			echo 'No GuiFont is set'
		endif
	else
		call GuiFont(a:fname, a:bang ==# '!')
	endif
endfunction
command! -nargs=? -bang Guifont call s:GuiFontCommand("<args>", "<bang>")
command! -nargs=? -bang GuiFont call s:GuiFontCommand("<args>", "<bang>")
