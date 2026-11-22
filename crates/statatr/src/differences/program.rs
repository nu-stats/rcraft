Guide {
    command: "program",
    about: "Defines a program the do-file then runs by name",
    points: &[
        p(
            "In R",
            "There is no R function for a program: RCraft runs its lines where it is called, \
             each translated as any line is, with locals of its own (`0', `1' ... and args), \
             temporary variables dropped when it returns, exit returning from it, and an \
             r-class program's return scalar and return local as r() afterwards. \
             marksample marks the rows its if and in keep, 0 where a variable of its \
             varlist is missing (unless novarlist), as Stata's.",
        ),
        p(
            "Not yet",
            "sclass, byable programs and program list are refused; a program with a line \
             that is not translated leaves r() unknown, and an e-class one e(). An e-class \
             program's ereturn lines are on the ereturn page.",
        ),
    ],
}
