Guide {
    command: "file",
    about: "Writes a text or binary file, and reads a text file line by line",
    points: &[p(
        "As Stata 19",
        "file open (write, with replace or append, text by default or binary), file write \
         (strings, _newline(n), _tab(n), _skip(n), _char(n), (exp) as display writes it, \
         %fmt (exp), and %1bu and %1bs bytes in binary files) and file close (or _all). \
         Text files take the system's new lines (\r\n on Windows), as Stata's do. The \
         errors stop where Stata's do (r(602), r(110), r(111)). seek, query and set are \
         not translated yet.",
    ),
    p(
        "file read",
        "file open h using f, read and then file read h name put the next line in the \
         local, without its end; r(status) says how the line ended (win, unix, mac, or none \
         for a last line without one) and r(eof) is 1, the local empty, past the last line, \
         so a while r(eof) == 0 loop goes through a file as in Stata. file open and close \
         leave r() as it was. Binary reads (formats) and read write together are not \
         translated yet.",
    )],
}
