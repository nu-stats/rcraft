* RCraft's Stata corpus (P2C.12y, batch B): made-up folders; Stata 19's results are in mkdir.recorded
* commands: mkdir rmdir
* mkdir and rmdir: a new folder, one that is there, a nested path whose parent is missing, a file with the name, a folder with something in it, and each syntax error
version 19
set linesize 255
capture mkdir mkdir_a
display "@ new " _rc
capture mkdir mkdir_a
display "@ again " _rc
capture mkdir mkdir_a/inner
display "@ inner " _rc
capture mkdir mkdir_b/sub/deep
display "@ noparent " _rc
capture mkdir "mkdir c d"
display "@ spaces " _rc
capture mkdir mkdir_a/
display "@ slash " _rc
capture mkdir
display "@ none " _rc
capture mkdir mkdir_x mkdir_y
display "@ two " _rc
capture mkdir mkdir_e, public
display "@ public " _rc
tempname fh
file open `fh' using mkdir_file, write replace
file close `fh'
capture mkdir mkdir_file
display "@ afile " _rc
capture rmdir mkdir_a
display "@ full " _rc
capture rmdir mkdir_nosuch
display "@ missing " _rc
capture rmdir mkdir_file
display "@ rmfile " _rc
capture rmdir
display "@ rmnone " _rc
capture rmdir mkdir_a/inner
display "@ rminner " _rc
capture rmdir mkdir_a
display "@ rmouter " _rc
capture rmdir "mkdir c d"
display "@ rmspaces " _rc
capture rmdir mkdir_e
display "@ rmpublic " _rc
erase mkdir_file
