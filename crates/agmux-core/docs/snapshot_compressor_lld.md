
## LLD-UX-TERMINAL-CELLS-104 (parser)
Resize the existing VT parser using `set_size`, retaining cursor, alternate
screen and input modes. Reconstructing a parser from visible contents loses
those states. The public screen-buffer regression checks cursor position,
dimensions and bracketed-paste mode after resize.
