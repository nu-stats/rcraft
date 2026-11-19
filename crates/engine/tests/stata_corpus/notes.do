* RCraft's Stata corpus (P2C.12y, batch A2): made-up data; Stata 19's results are in notes.recorded
* commands: note notes
* (note and notes: adding to the data and to a variable, the count kept in note0 and the numbers that stay after a drop, drop (one, a range, all, _all, the top one), replace, renumber, search, a note with commas and if/in words, an empty note, the errors with their return codes, and the notes kept in a .dta file)
version 19
set linesize 255
clear
set obs 2
generate x = _n
generate y = 2 * _n
generate z = 3
note: Made up.
note: A second note.
note x: x counts.
notes
notes x
notes _dta
notes y
notes list
notes list x
display "@ add " (`_dta[note0]' == 2) " " (`x[note0]' == 1) " " ("`x[note1]'" == "x counts.") " " ("`_dta[note1]'" == "Made up.") " " ("`_dta[note2]'" == "A second note.") " " ("`y[note0]'" == "")
notes drop _dta in 1
display "@ drop1 " `_dta[note0]' " " ("`_dta[note1]'" == "") " " ("`_dta[note2]'" == "A second note.")
notes replace x in 1: x is now.
display "@ replace " ("`x[note1]'" == "x is now.") " " `x[note0]'
notes replace z in 5: nope
display "@ replace_none " ("`z[note0]'" == "")
note x: more
note x: more2
notes drop x in 2
display "@ gap " `x[note0]' " " ("`x[note2]'" == "") " " ("`x[note3]'" == "more2")
note x: more3
display "@ after " `x[note0]' " " ("`x[note4]'" == "more3")
notes drop x in 4
display "@ droptop " `x[note0]'
note x: again
display "@ again " `x[note0]' " " ("`x[note4]'" == "again")
notes drop x in 1/4
display "@ droprange " ("`x[note0]'" == "") " " ("`x[note1]'" == "") " " ("`x[note3]'" == "")
note x: five
display "@ restart " `x[note0]' " " ("`x[note1]'" == "five")
notes drop x
display "@ dropall " ("`x[note0]'" == "") " " ("`x[note1]'" == "")
notes drop y
notes drop x y in 1
note y: abc, def
note z: go if x in 3
display "@ words " ("`y[note1]'" == "abc, def") " " ("`z[note1]'" == "go if x in 3")
notes search abc
notes search "def"
notes search nothingelse
notes y: second form
display "@ second " `y[note0]' " " ("`y[note2]'" == "second form")
notes: dta second form
display "@ dta_form " ("`_dta[note3]'" == "dta second form") " " `_dta[note0]'
notes drop _dta in 1/2
display "@ dropdta " `_dta[note0]' " " ("`_dta[note2]'" == "") " " ("`_dta[note3]'" == "dta second form")
notes renumber _dta
display "@ renumber " `_dta[note0]' " " ("`_dta[note1]'" == "dta second form") " " ("`_dta[note3]'" == "")
note y:
display "@ empty " `y[note0]'
notes y:
display "@ empty2 " `y[note0]'
quietly notes y
note
notes z y
capture notes nosuch
display "@ rc_notes_nosuch " _rc
capture note nosuch: abc
display "@ rc_note_nosuch " _rc
capture notes drop nosuch
display "@ rc_drop_nosuch " _rc
capture notes drop x in 9
display "@ rc_drop_none " _rc
note x: for the file
notes y
save notes_rt, replace
clear
use notes_rt, clear
display "@ reread " ("`x[note1]'" == "for the file") " " `x[note0]' " " ("`y[note1]'" == "abc, def") " " ("`z[note1]'" == "go if x in 3") " " `_dta[note0]'
notes
notes drop _all
display "@ dropall2 " ("`_dta[note0]'" == "") " " ("`x[note0]'" == "") " " ("`y[note0]'" == "") " " ("`z[note0]'" == "")
erase notes_rt.dta
