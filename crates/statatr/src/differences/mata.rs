Guide {
    command: "mata",
    about: "Stata's matrix language",
    points: &[p(
        "Not translated",
        "Mata is not translated: a mata block or a mata: line is refused with the reason.              The one exception is mata: mata clear, which outreg do-files use to start over:              it empties outreg's tables, the only thing RCraft keeps where Stata keeps Mata's              memory.",
    )],
}
