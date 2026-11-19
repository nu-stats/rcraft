* RCraft's Stata corpus (P2C.12d continued): made-up data; Stata 19's results are in functions_f.recorded
* functions: scalar bsubstr chiprob fprob index invfprob invnorm itrim match matrix normprob plural proper regexm regexr regexs reverse strpos stritrim strproper strreverse strrpos subinword udsubstr word wordcount
* (the functions of batch F: word() and wordcount() (blanks only separate; tabs, new lines and quotes are part of a word), strproper() and stritrim() (ASCII letters and blanks), strrpos(), strpos() and strreverse() on bytes, subinword(), regexr() (the new text as it stands; an invalid expression leaves the string), regexm() with regexs(), plural(), bsubstr() and udsubstr(), matrix(), match(), and the old names normprob(), invnorm(), chiprob(), fprob(), invfprob(), proper(), itrim(), index(), reverse(); cofc() in lower case is no Stata function)
* tolerance: 1e-8
version 19
clear
set linesize 255
* word() and wordcount()
display "@ word [" subinstr(word("a b  c", 1), " ", "_", .) "] [" subinstr(word("a b  c", 2), " ", "_", .) "] [" subinstr(word("a b  c", 3), " ", "_", .) "] [" subinstr(word("a b  c", 4), " ", "_", .) "] [" subinstr(word("a b  c", 0), " ", "_", .) "] [" subinstr(word("a b  c", -1), " ", "_", .) "] [" subinstr(word("a b  c", -2), " ", "_", .) "] [" subinstr(word("a b  c", -3), " ", "_", .) "] [" subinstr(word("a b  c", -4), " ", "_", .) "]"
display "@ word_n [" subinstr(word("a b  c", 1.9), " ", "_", .) "] [" subinstr(word("a b  c", 2.5), " ", "_", .) "] [" subinstr(word("a b  c", -1.5), " ", "_", .) "] [" subinstr(word("a b  c", .), " ", "_", .) "] [" subinstr(word("a b  c", .a), " ", "_", .) "] [" subinstr(word("a b  c", 1e10), " ", "_", .) "] [" subinstr(word("a b  c", -1e10), " ", "_", .) "]"
display "@ word_blanks [" subinstr(word("", 1), " ", "_", .) "] [" subinstr(word("   ", 1), " ", "_", .) "] [" subinstr(word("  a  b ", 1), " ", "_", .) "] [" subinstr(word("  a  b ", 2), " ", "_", .) "] [" subinstr(word("  a  b ", -1), " ", "_", .) "] [" subinstr(word("one two three four five six", 6), " ", "_", .) "] [" subinstr(word("one two three four five six", -6), " ", "_", .) "] [" subinstr(word("one two three four five six", -7), " ", "_", .) "]"
display "@ word_tab " length(word("a" + char(9) + "b c", 1)) " " length(word("a" + char(9) + "b c", 2)) " " length(word("a" + char(10) + "b c", 1)) " " length(word(char(9) + "a b", 1)) " [" subinstr(word("a,b c;d e", 1), " ", "_", .) "] [" subinstr(word("a,b c;d e", 2), " ", "_", .) "]"
display "@ word_quote [" subinstr(word(`"a "b c" d"', 2), " ", "_", .) "] [" subinstr(word(`"a "b c" d"', 3), " ", "_", .) "] [" subinstr(word(`""a b" c"', 1), " ", "_", .) "]"
display "@ word_utf [" subinstr(word("é ñ ü", 2), " ", "_", .) "] [" subinstr(word("é ñ ü", -1), " ", "_", .) "]"
display "@ wordcount " wordcount("a b  c") " " wordcount("") " " wordcount("   ") " " wordcount("a" + char(9) + "b") " " wordcount("a" + char(10) + "b") " " wordcount(`"a "b c" d"') " " wordcount("  a  b  ")
* strproper(), stritrim()
display "@ proper [" subinstr(strproper("hello wORLD"), " ", "_", .) "] [" subinstr(strproper("o'neil mc-donald"), " ", "_", .) "] [" subinstr(strproper("123abc def"), " ", "_", .) "] [" subinstr(strproper("a1b2 c3d"), " ", "_", .) "] [" subinstr(strproper(" multiple  spaces "), " ", "_", .) "] [" subinstr(strproper("x_y z.w"), " ", "_", .) "] [" subinstr(strproper("it's"), " ", "_", .) "] [" subinstr(strproper(""), " ", "_", .) "] [" subinstr(strproper("."), " ", "_", .) "]"
display "@ proper_utf [" subinstr(strproper("ÉLAN vital ñandú"), " ", "_", .) "] [" subinstr(strproper("MARY-ANN SMITH"), " ", "_", .) "] [" subinstr(strproper("one,two;three"), " ", "_", .) "] [" subinstr(strproper("(paren) [brack]"), " ", "_", .) "] [" subinstr(strproper("ß straße"), " ", "_", .) "] [" subinstr(strproper("ab12cd"), " ", "_", .) "] [" subinstr(strproper("_a"), " ", "_", .) "] [" subinstr(strproper("a'B"), " ", "_", .) "] [" subinstr(strproper("a-b-c"), " ", "_", .) "]"
display "@ proper_tab " length(strproper("a" + char(9) + "bb")) " " (strproper("a" + char(9) + "bb") == "A" + char(9) + "Bb") " " (strproper("a" + char(10) + "bb") == "A" + char(10) + "Bb") " " (proper("heLLo woRLD") == "Hello World")
display "@ itrim [" subinstr(stritrim("a   b"), " ", "_", .) "] [" subinstr(stritrim("  a  b  "), " ", "_", .) "] [" subinstr(stritrim("x   "), " ", "_", .) "] [" subinstr(stritrim("   x"), " ", "_", .) "] [" subinstr(stritrim(""), " ", "_", .) "] [" subinstr(stritrim("    "), " ", "_", .) "] [" subinstr(stritrim("a b"), " ", "_", .) "] [" subinstr(stritrim(`"a  "b  c"  d"'), " ", "_", .) "] [" subinstr(stritrim("a  b   c    d"), " ", "_", .) "]"
display "@ itrim_tab " length(stritrim("a" + char(9) + char(9) + "b")) " " length(stritrim("a" + char(10) + char(10) + "b")) " " length(stritrim("a  " + char(9) + " b")) " " length(stritrim("  a  b  ")) " " length(stritrim("   ")) " " length(stritrim(char(9) + char(9) + "a")) " " length(stritrim("a" + char(13) + char(13) + "b")) " " (itrim("x   y") == "x y")
* strrpos(), strpos(), strreverse()
display "@ rpos " strrpos("abcabc", "b") " " strrpos("abcabc", "z") " " strrpos("abcabc", "") " " strrpos("", "a") " " strrpos("", "") " " strrpos("aaa", "aa") " " strrpos("abc", "abcd") " " strrpos("abcabc", "abc") " " strrpos("é x", "x") " " strrpos("éé", "é") " " strrpos("a.b.c", ".") " " strrpos("ABC", "b") " " strrpos("abcabc", "bc") " " strrpos("é", "") " " strrpos("abc", "")
display "@ pos " strpos("abcabc", "b") " " strpos("abcabc", "") " " strpos("", "") " " strpos("", "a") " " strpos("é x", "x") " " strpos("abc", "c") " " strpos("abc", "abcd") " " strpos("é", "") " " strpos("aaa", "aa") " " index("abc", "b") " " index("abc", "z")
display "@ rev [" subinstr(strreverse("abc"), " ", "_", .) "] [" subinstr(strreverse(""), " ", "_", .) "] [" subinstr(strreverse("a b  c "), " ", "_", .) "] " length(strreverse("café")) " " (strreverse(strreverse("é")) == "é") " " length(strreverse("é")) " " (strreverse(strreverse("aéb")) == "aéb") " " (reverse("abc") == "cba")
* subinword()
display "@ subword1 [" subinstr(subinword("a b c", "b", "x", .), " ", "_", .) "] [" subinstr(subinword("ab b b", "b", "x", .), " ", "_", .) "] [" subinstr(subinword("ab b b", "b", "x", 1), " ", "_", .) "] [" subinstr(subinword("ab b b", "b", "x", 2), " ", "_", .) "] [" subinstr(subinword("ab b b", "b", "x", 3), " ", "_", .) "] [" subinstr(subinword("ab b b", "b", "x", 0), " ", "_", .) "] [" subinstr(subinword("ab b b", "b", "x", -1), " ", "_", .) "] [" subinstr(subinword("ab b b", "b", "x", 1.9), " ", "_", .) "]"
display "@ subword2 [" subinstr(subinword("a,b c", "b", "x", .), " ", "_", .) "] [" subinstr(subinword("a,b c", "a,b", "x", .), " ", "_", .) "] [" subinstr(subinword("a b c", "b c", "x", .), " ", "_", .) "] [" subinstr(subinword("a b c", "a b c", "x", .), " ", "_", .) "] [" subinstr(subinword("a b c", "", "x", .), " ", "_", .) "] [" subinstr(subinword("", "", "x", .), " ", "_", .) "] [" subinstr(subinword("a b c", "b", "", .), " ", "_", .) "] [" subinstr(subinword("a b c", "b", "x y z", .), " ", "_", .) "]"
display "@ subword3 [" subinstr(subinword("a  b   c", "b", "x", .), " ", "_", .) "] [" subinstr(subinword("a  b   c", "a  b", "x", .), " ", "_", .) "] [" subinstr(subinword("a b", "a   b", "x", .), " ", "_", .) "] [" subinstr(subinword(" b ", "b", "x", .), " ", "_", .) "] [" subinstr(subinword("b b", "b", "bb", .), " ", "_", .) "] [" subinstr(subinword("aXb X", "X", "y", .), " ", "_", .) "] [" subinstr(subinword("b", "b", "x", .), " ", "_", .) "]"
display "@ subword4 [" subinstr(subinword("a" + char(9) + "b b c", "b", "x", .), " ", "_", .) "] [" subinstr(subinword("a.b c", "b", "x", .), " ", "_", .) "] [" subinstr(subinword("a-b", "b", "x", .), " ", "_", .) "] [" subinstr(subinword("é b", "b", "x", .), " ", "_", .) "] [" subinstr(subinword("éb b", "b", "x", .), " ", "_", .) "] [" subinstr(subinword("a b c", " b", "x", .), " ", "_", .) "] [" subinstr(subinword("a b c", "b ", "x", .), " ", "_", .) "] [" subinstr(subinword("a  b", "a ", "x", .), " ", "_", .) "] [" subinstr(subinword("a b", "a b ", "x", .), " ", "_", .) "]"
display "@ subword5 [" subinstr(subinword("a b c", "b", "x", .a), " ", "_", .) "] [" subinstr(subinword("a b c", "b", "x", 1e10), " ", "_", .) "] [" subinstr(subinword("b b b b", "b", "bb", 2), " ", "_", .) "] [" subinstr(subinword("a b", "a b", "a b a b", .), " ", "_", .) "] [" subinstr(subinword("a b a b", "a b", "z", 2), " ", "_", .) "] [" subinstr(subinword("aa a aa a", "a", "b", .), " ", "_", .) "] [" subinstr(subinword("a b b", "b", "x", 2147483647), " ", "_", .) "] [" subinstr(subinword("a b b", "b", "x", 2147483648), " ", "_", .) "] [" subinstr(subinword("a b b", "b", "x", 1e9), " ", "_", .) "]"
* regexr()
display "@ regexr1 [" subinstr(regexr("abc", "b", "x"), " ", "_", .) "] [" subinstr(regexr("abcabc", "b", "x"), " ", "_", .) "] [" subinstr(regexr("abc", "z", "x"), " ", "_", .) "] [" subinstr(regexr("abc", "", "x"), " ", "_", .) "] [" subinstr(regexr("", "", "x"), " ", "_", .) "] [" subinstr(regexr("", "a*", "x"), " ", "_", .) "] [" subinstr(regexr("abc", "b*", "x"), " ", "_", .) "] [" subinstr(regexr("abc", "^", "x"), " ", "_", .) "] [" subinstr(regexr("abc", "$", "x"), " ", "_", .) "]"
display "@ regexr2 [" subinstr(regexr("abc", "(b)", "[\1]"), " ", "_", .) "] [" subinstr(regexr("abc", "(b)", "[\0]"), " ", "_", .) "] [" subinstr(regexr("abc", "b", "\1"), " ", "_", .) "] [" subinstr(regexr("abc", "b", "\\"), " ", "_", .) "] [" subinstr(regexr("abc", "b", "$1"), " ", "_", .) "] [" subinstr(regexr("abc", "b", "&"), " ", "_", .) "]"
display "@ regexr3 [" subinstr(regexr("a.c", ".", "x"), " ", "_", .) "] [" subinstr(regexr("a.c", "\.", "x"), " ", "_", .) "] [" subinstr(regexr("Abc", "[a-z]+", "x"), " ", "_", .) "] [" subinstr(regexr("abc", "B", "x"), " ", "_", .) "] [" subinstr(regexr("abc123", "[0-9]+", ""), " ", "_", .) "] [" subinstr(regexr("abc  def", " +", "_"), " ", "_", .) "] [" subinstr(regexr("abc", "a|b", "x"), " ", "_", .) "] [" subinstr(regexr("abc", "(a|b)+", "x"), " ", "_", .) "] [" subinstr(regexr("aaa", "a*?", "x"), " ", "_", .) "]"
display "@ regexr4 [" subinstr(regexr("abc", "\w", "x"), " ", "_", .) "] [" subinstr(regexr("abc def", "\s", "_"), " ", "_", .) "] [" subinstr(regexr("abc", "[[:alpha:]]+", "x"), " ", "_", .) "] [" subinstr(regexr("abc", "\d", "x"), " ", "_", .) "] [" subinstr(regexr("a1c", "\d", "x"), " ", "_", .) "] [" subinstr(regexr("abc", "(?i)B", "x"), " ", "_", .) "] [" subinstr(regexr("ab", "a(?=b)", "x"), " ", "_", .) "] [" subinstr(regexr("abc", "\bb", "x"), " ", "_", .) "]"
display "@ regexr5 [" subinstr(regexr("abc", "(", "x"), " ", "_", .) "] [" subinstr(regexr("abc", "[", "x"), " ", "_", .) "] [" subinstr(regexr("abc", "*", "x"), " ", "_", .) "] [" subinstr(regexr("a*c", "*", "x"), " ", "_", .) "] [" subinstr(regexr("a*c", "\*", "x"), " ", "_", .) "] [" subinstr(regexr("a(c", "(", "x"), " ", "_", .) "] [" subinstr(regexr("a)c", ")", "x"), " ", "_", .) "] [" subinstr(regexr("a]c", "]", "x"), " ", "_", .) "] [" subinstr(regexr("a{c", "{", "x"), " ", "_", .) "] [" subinstr(regexr("a?c", "?", "x"), " ", "_", .) "] [" subinstr(regexr("a+c", "+", "x"), " ", "_", .) "]"
display "@ regexr6 [" subinstr(regexr("ab", "a|ab", "x"), " ", "_", .) "] [" subinstr(regexr("aab", "a{2}", "x"), " ", "_", .) "] [" subinstr(regexr("aa", "(a)\1", "x"), " ", "_", .) "] [" subinstr(regexr("aaa", "a+?", "x"), " ", "_", .) "] [" subinstr(regexr("a b", "\<b", "x"), " ", "_", .) "] [" subinstr(regexr("foo.bar", "[.]", "x"), " ", "_", .) "] [" subinstr(regexr("ab", "a|ab|abc", "x"), " ", "_", .) "] [" subinstr(regexr("abc", "a|abc", "x"), " ", "_", .) "] [" subinstr(regexr("a+b", "a\+b", "x"), " ", "_", .) "] [" subinstr(regexr("a+b", "a+b", "x"), " ", "_", .) "] [" subinstr(regexr("ab", "(?:a)b", "x"), " ", "_", .) "] [" subinstr(regexr("AB", "[[:lower:]]", "x"), " ", "_", .) "]"
display "@ regexr_utf [" subinstr(regexr("éb", "b", "x"), " ", "_", .) "] [" subinstr(regexr("aéb", "é", "x"), " ", "_", .) "] " length(regexr("é", ".", "x")) " " length(regexr("é.c", ".", "x"))
* regexm() and regexs()
display "@ regexm1 " regexm("abc123", "([a-z]+)([0-9]+)") " [" subinstr(regexs(0), " ", "_", .) "] [" subinstr(regexs(1), " ", "_", .) "] [" subinstr(regexs(2), " ", "_", .) "] [" subinstr(regexs(3), " ", "_", .) "] [" subinstr(regexs(9), " ", "_", .) "] [" subinstr(regexs(10), " ", "_", .) "] [" subinstr(regexs(-1), " ", "_", .) "] [" subinstr(regexs(.), " ", "_", .) "] [" subinstr(regexs(1.7), " ", "_", .) "]"
display "@ regexm2 " regexm("abc", "x") " [" subinstr(regexs(0), " ", "_", .) "] [" subinstr(regexs(1), " ", "_", .) "]"
display "@ regexm3 " regexm("abc", "(x)?b") " [" subinstr(regexs(0), " ", "_", .) "] [" subinstr(regexs(1), " ", "_", .) "]"
display "@ regexm4 " regexm("ab", "(a)(b)") " " regexm("zz", "(q)") " [" subinstr(regexs(1), " ", "_", .) "]"
display "@ regexm5 " regexm("", "(a*)") " [" subinstr(regexs(0), " ", "_", .) "] [" subinstr(regexs(1), " ", "_", .) "] " regexm("ab", "a|ab") " [" subinstr(regexs(0), " ", "_", .) "] " regexm("abcd", "(a|ab)(c|bcd)") " [" subinstr(regexs(0), " ", "_", .) "] [" subinstr(regexs(1), " ", "_", .) "] [" subinstr(regexs(2), " ", "_", .) "]"
display "@ regexm6 " regexm("a1", "\d") " " regexm("a1", "(?i)A") " " regexm("a", "(?=a)") " " regexm("a", "\Aa\z") " " regexm("é", "^.$") " " regexm("é", "^..$") " " regexm("É", "é") " " regexm("a", "[[:alpha:]]")
display "@ regexm7 [" subinstr(regexs(0), " ", "_", .) "]"
set obs 4
generate str20 phone = "ab 555-1234 x" in 1
replace phone = "none" in 2
replace phone = "10 200-3000" in 3
replace phone = "q 7-8" in 4
generate str10 area = regexs(1) if regexm(phone, "([0-9]+)-([0-9]+)")
generate str10 rest = regexs(2) if regexm(phone, "([0-9]+)-([0-9]+)")
generate hit = regexm(phone, "[0-9]+-[0-9]+")
forvalues i = 1/4 {
  display "@ regexm_rows `i' [" area[`i'] "] [" rest[`i'] "] " hit[`i']
}
replace area = regexs(0) if regexm(phone, "[a-z]") & _n < 3
forvalues i = 1/4 {
  display "@ regexm_rows2 `i' [" area[`i'] "]"
}
* plural()
display "@ plural1 [" subinstr(plural(2, "cat"), " ", "_", .) "] [" subinstr(plural(1, "cat"), " ", "_", .) "] [" subinstr(plural(0, "cat"), " ", "_", .) "] [" subinstr(plural(1.5, "cat"), " ", "_", .) "] [" subinstr(plural(-1, "cat"), " ", "_", .) "] [" subinstr(plural(-2, "cat"), " ", "_", .) "] [" subinstr(plural(., "cat"), " ", "_", .) "] [" subinstr(plural(.a, "cat"), " ", "_", .) "] [" subinstr(plural(2, "bus"), " ", "_", .) "] [" subinstr(plural(2, "city"), " ", "_", .) "] [" subinstr(plural(2, ""), " ", "_", .) "] [" subinstr(plural(1, ""), " ", "_", .) "] [" subinstr(plural(2, "Cat"), " ", "_", .) "] [" subinstr(plural(2, "cat "), " ", "_", .) "]"
display "@ plural2 [" subinstr(plural(2, "child", "ren"), " ", "_", .) "] [" subinstr(plural(1, "child", "ren"), " ", "_", .) "] [" subinstr(plural(2, "mouse", "-ouse"), " ", "_", .) "] [" subinstr(plural(2, "wife", "-fe"), " ", "_", .) "] [" subinstr(plural(2, "mouse", "mice"), " ", "_", .) "] [" subinstr(plural(2, "ox", "en"), " ", "_", .) "] [" subinstr(plural(2, "city", "-y"), " ", "_", .) "] [" subinstr(plural(2, "city", "-y+ies"), " ", "_", .) "] [" subinstr(plural(2, "cat", ""), " ", "_", .) "] [" subinstr(plural(2, "cat", "-t"), " ", "_", .) "] [" subinstr(plural(2, "cat", "-cat"), " ", "_", .) "] [" subinstr(plural(2, "cat", "-xcat"), " ", "_", .) "] [" subinstr(plural(2, "cat", "+s"), " ", "_", .) "] [" subinstr(plural(2, "cat", "-"), " ", "_", .) "]"
display "@ plural3 [" subinstr(plural(2, "fox", "es"), " ", "_", .) "] [" subinstr(plural(2, "child", "children"), " ", "_", .) "] [" subinstr(plural(2, "cat", "-t+ts"), " ", "_", .) "] [" subinstr(plural(0, "child", "ren"), " ", "_", .) "] [" subinstr(plural(., "child", "ren"), " ", "_", .) "] [" subinstr(plural(-1, "child", "ren"), " ", "_", .) "]"
* bsubstr(), udsubstr() and substr()
display "@ bsub1 [" subinstr(bsubstr("abcdef", 2, 3), " ", "_", .) "] [" subinstr(bsubstr("abcdef", 0, 3), " ", "_", .) "] [" subinstr(bsubstr("abcdef", -2, 2), " ", "_", .) "] [" subinstr(bsubstr("abcdef", -2, .), " ", "_", .) "] [" subinstr(bsubstr("abcdef", 2, .), " ", "_", .) "] [" subinstr(bsubstr("abcdef", 2, 0), " ", "_", .) "] [" subinstr(bsubstr("abcdef", 2, -1), " ", "_", .) "] [" subinstr(bsubstr("abcdef", 7, 1), " ", "_", .) "] [" subinstr(bsubstr("abcdef", 6, 5), " ", "_", .) "] [" subinstr(bsubstr("abcdef", -10, 3), " ", "_", .) "] [" subinstr(bsubstr("abcdef", -10, 8), " ", "_", .) "] [" subinstr(bsubstr("", 1, 1), " ", "_", .) "]"
display "@ bsub2 [" subinstr(bsubstr("abcdef", ., 2), " ", "_", .) "] [" subinstr(bsubstr("abcdef", 2, .a), " ", "_", .) "] [" subinstr(bsubstr("abcdef", 1.9, 2.9), " ", "_", .) "] [" subinstr(bsubstr("abcdef", -1.5, 1), " ", "_", .) "] [" subinstr(bsubstr("abcdef", -6, 1), " ", "_", .) "] [" subinstr(bsubstr("abcdef", -7, 2), " ", "_", .) "] [" subinstr(bsubstr("abcdef", 1, 100), " ", "_", .) "] [" subinstr(bsubstr("abcdef", 1e10, 2), " ", "_", .) "]"
display "@ bsub3 " length(bsubstr("éa", 1, 1)) " " length(bsubstr("éa", 2, 2)) " " length(bsubstr("éa", 1, 2)) " " (bsubstr("éa", 1, 2) == "é") " " (bsubstr("éa", 3, 1) == "a")
display "@ udsub1 [" subinstr(udsubstr("éaü", 2, 2), " ", "_", .) "] [" subinstr(udsubstr("éaü", -1, 1), " ", "_", .) "] [" subinstr(udsubstr("éaü", 0, 1), " ", "_", .) "] [" subinstr(udsubstr("éaü", 2, .), " ", "_", .) "] [" subinstr(udsubstr("éaü", 1, 5), " ", "_", .) "] [" subinstr(udsubstr("éaü", -3, 2), " ", "_", .) "] [" subinstr(udsubstr("éaü", -5, 2), " ", "_", .) "] [" subinstr(udsubstr("éaü", 1, 1), " ", "_", .) "]"
display "@ udsub2 [" subinstr(udsubstr("日本語", 1, 2), " ", "_", .) "] [" subinstr(udsubstr("日本語", 1, 1), " ", "_", .) "] [" subinstr(udsubstr("日本語", 2, 2), " ", "_", .) "] [" subinstr(udsubstr("日本語", 2, 3), " ", "_", .) "] [" subinstr(udsubstr("a日b", 2, 1), " ", "_", .) "] [" subinstr(udsubstr("a日b", 2, 2), " ", "_", .) "] [" subinstr(udsubstr("a日b", 3, 1), " ", "_", .) "] [" subinstr(udsubstr("a日b", 4, 1), " ", "_", .) "]"
* matrix()
matrix M = (1, 2 \ 3, 4)
scalar sc = 7
display "@ matrix " matrix(3) " " matrix(M[1,2]) " " matrix(2 + M[2,1]) " " matrix(sc) " " matrix(.) " " matrix(M[1,1] * 5) " [" matrix("a") "]"
generate zz = matrix(M[2,2]) in 1
generate ww = matrix(sc) in 1
display "@ matel_out " M[3,3] " " M[0,1] " " M[1,.] " " M[-1,1] " " M[1,2.7] " " M[2.9,1] " " matrix(M[3,3]) " " matrix(M[3,3] + 1) " " scalar(M[5,5])
display "@ scalar " scalar(sc) " " scalar(4) " " scalar(2 + 3) " [" scalar("a") "] " scalar(sc + 1) " " scalar(M[2,2]) " " scalar(.)
display "@ matrix_gen " zz[1] " " zz[2] " " ww[1]
* match() and the old names
display "@ match " match("abc", "a*") " " match("abc", "?b?") " " match("abc", "b*") " " match("", "*") " " match("abc", "ABC")
foreach x in -3 -1 0 .1 1.96 5 38 -38 40 . {
  display "@ normprob `x' " %24.17g normprob(`x') " " %24.17g normal(`x')
}
foreach p in -1 0 1e-300 1e-10 .025 .5 .975 1 1.5 . {
  display "@ invnorm `p' " %24.17g invnorm(`p') " " %24.17g invnormal(`p')
}
foreach d in -1 0 .5 1 3 100 1e6 1e17 2e17 3e17 . {
  foreach x in -1 0 .5 3 7.81 100 1e6 . {
    display "@ chiprob `d' `x' " %24.17g chiprob(`d', `x')
  }
}
foreach a in -1 0 .5 1 2 10 1e6 . {
  foreach b in -1 0 .5 1 10 1e6 . {
    foreach f in -1 0 .5 4.1 30 . {
      display "@ fprob `a' `b' `f' " %24.17g fprob(`a', `b', `f')
    }
  }
}
foreach a in 0 .5 1 2 10 1e6 . {
  foreach b in 0 1 10 1e6 . {
    foreach p in -1 0 1e-12 .05 .5 .99 1 1.5 . {
      display "@ invfprob `a' `b' `p' " %24.17g invfprob(`a', `b', `p')
    }
  }
}
