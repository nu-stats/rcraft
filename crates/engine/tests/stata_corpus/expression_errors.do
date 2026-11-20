* RCraft's Stata corpus (P2C.12ab): made-up data; Stata 19's return codes are in expression_errors.recorded
* (an expression Stata does not read, under capture: r(198) for an operator at the end, a token after the expression or a text it cannot name, r(132) for unpaired parentheses; the line changes nothing and the do-file goes on)
* tolerance: 0
version 19
set linesize 255
capture display 4 \\ 5
display "@ k1 " _rc
capture display 4 \ 5
display "@ k2 " _rc
capture display 4 \5
display "@ k3 " _rc
capture display 4\ 5
display "@ k4 " _rc
capture display 4 \ 5 + 6
display "@ k5 " _rc
capture display 4 # 5
display "@ k6 " _rc
capture display 4 @ 5
display "@ k7 " _rc
capture display 4 @5 6
display "@ k8 " _rc
capture display 4 + @ 5
display "@ k9 " _rc
capture display @ 5
display "@ k10 " _rc
capture display (4 @ 5)
display "@ k11 " _rc
capture display 4 ; 5
display "@ k12 " _rc
capture display 1 + 2 \ 3
display "@ k13 " _rc
capture display 1+2 \ 3
display "@ k14 " _rc
capture display 1 +  2   \  3
display "@ k15 " _rc
capture display "a" \ 3
display "@ k16 " _rc
capture display 4 \\
display "@ k17 " _rc
capture display 5-
display "@ k18 " _rc
capture display 5 -
display "@ k19 " _rc
capture display 5 -
display "@ k20 " _rc
capture display 2 * (3 -)
display "@ k21 " _rc
capture display 2 * (3 - ) + 1
display "@ k22 " _rc
capture display 1e
display "@ k23 " _rc
capture display 2 + 1e 3
display "@ k24 " _rc
capture display 1,2
display "@ k25 " _rc
capture display 3 ^
display "@ k26 " _rc
capture display 3 + * 4
display "@ k27 " _rc
capture display (3 + 4
display "@ k28 " _rc
capture display 3 + 4)
display "@ k29 " _rc
capture display 3 = 4
display "@ k30 " _rc
capture display 3 == 4 \ 5
display "@ k31 " _rc
capture display 3 & | 4
display "@ k32 " _rc
capture display 7 ! 4
display "@ k33 " _rc
capture display 7 !
display "@ k34 " _rc
capture scalar s = 4 \ 5
display "@ k35 " _rc
capture scalar s = 4 @ 5
display "@ k36 " _rc
capture scalar s = 4 5
display "@ k37 " _rc
capture scalar s = 5 -
display "@ k38 " _rc
capture scalar s = 1 +  2   \  3
display "@ k39 " _rc
capture scalar s = (4 @ 5)
display "@ k40 " _rc
capture display 4 + (3 + * 4)
display "@ k41 " _rc
capture display 4 \5+6
display "@ k42 " _rc
capture display "x" @ 5
display "@ k43 " _rc
capture display 4 #5
display "@ k44 " _rc
capture display 3 > | 4
display "@ k45 " _rc
capture display (7 ! 4)
display "@ k46 " _rc
capture display (3 & | 4)
display "@ k47 " _rc
capture display 1, 2, 3
display "@ k48 " _rc
capture display 1 , "a"
display "@ k49 " _rc
capture display = 4
display "@ k50 " _rc
capture display 1 = = 2
display "@ k51 " _rc
capture display ((3 + 4)
display "@ k52 " _rc
capture display (3 + 4))
display "@ k53 " _rc
capture scalar s = 7 ! 4
display "@ k54 " _rc
capture scalar s = 3 + * 4
display "@ k55 " _rc
capture scalar s = 4 @5
display "@ k56 " _rc
capture scalar s = 4 \\ 5
display "@ k57 " _rc
capture scalar s = (3 + 4
display "@ k58 " _rc
capture scalar s = 3 + 4)
display "@ k59 " _rc
capture scalar s = 3 & | 4
display "@ k60 " _rc
capture scalar s = 2 + 1e 3
display "@ k61 " _rc
clear
set obs 1
capture generate y = 4 \ 5
display "@ k62 " _rc
capture generate y = 4 @ 5
display "@ k63 " _rc
capture generate y = 4 5
display "@ k64 " _rc
capture generate y = 5 -
display "@ k65 " _rc
capture generate y = 1 +  2   \  3
display "@ k66 " _rc
capture generate y = 2 * (3 - ) + 1
display "@ k67 " _rc
clear
set obs 3
generate x = _n
capture generate y = x +
display "@ k1001 " _rc
capture generate y = x *
display "@ k1002 " _rc
capture generate y = x ==
display "@ k1003 " _rc
capture generate y = x &
display "@ k1004 " _rc
capture generate y = -
display "@ k1005 " _rc
capture generate y = !
display "@ k1006 " _rc
capture replace x = x -
display "@ k1007 " _rc
capture assert x >
display "@ k1008 " _rc
capture count if x <
display "@ k1009 " _rc
capture local m = 3 +
display "@ k1010 " _rc
capture scalar s = "a" +
display "@ k1011 " _rc
capture scalar s = (3 +) + 1
display "@ k1012 " _rc
capture scalar s = 3 + +
display "@ k1013 " _rc
capture generate y = x 56
display "@ k1014 " _rc
capture generate y = x abc
display "@ k1015 " _rc
capture generate y = x "s"
display "@ k1016 " _rc
capture generate y = x (2)
display "@ k1017 " _rc
capture generate y = 2 x
display "@ k1019 " _rc
capture generate y = 1.5 .
display "@ k1020 " _rc
capture generate y = x , 2
display "@ k1021 " _rc
capture generate y = x ; 2
display "@ k1022 " _rc
capture generate y = x $ 2
display "@ k1023 " _rc
capture generate y = x = 2
display "@ k1024 " _rc
capture generate y = x == 2 3
display "@ k1025 " _rc
capture replace x = x 1
display "@ k1026 " _rc
capture assert x 1
display "@ k1027 " _rc
capture count if x 1
display "@ k1028 " _rc
capture local m = 3 4
display "@ k1029 " _rc
capture display (3 4)
display "@ k1030 " _rc
capture scalar s = "a" "b"
display "@ k1031 " _rc
capture scalar s = 3 4 5
display "@ k1032 " _rc
capture generate y = (x + 1
display "@ k1033 " _rc
capture generate y = x + 1)
display "@ k1034 " _rc
capture generate y = x[1
display "@ k1035 " _rc
capture generate y = x]
display "@ k1036 " _rc
capture generate y = (x[1)
display "@ k1037 " _rc
capture generate y = (x + 1]
display "@ k1038 " _rc
capture generate y = ")" + string(x)
display "@ k1039 " _rc
capture drop y
capture generate y = `"("'
display "@ k1040 " _rc
capture drop y
capture replace x = (x
display "@ k1041 " _rc
capture assert (x > 0
display "@ k1042 " _rc
capture count if x > 0)
display "@ k1043 " _rc
capture local m = (3
display "@ k1044 " _rc
capture display max(1, 2
display "@ k1045 " _rc
capture display max(1, 2))
display "@ k1046 " _rc
capture display ")" (1
display "@ k1047 " _rc
capture display "a" 3 + 4)
display "@ k1048 " _rc
capture scalar s = )(
display "@ k1049 " _rc
capture scalar s = ((1)
display "@ k1050 " _rc
capture display "(" ")" 1
display "@ k1051 " _rc
capture display max(1 @ 2, 3)
display "@ k1052 " _rc
capture display max(1, 2 @ 3)
display "@ k1053 " _rc
capture display max(1 2, 3)
display "@ k1054 " _rc
capture display max(1 -, 3)
display "@ k1055 " _rc
capture display max(1, 3 +)
display "@ k1056 " _rc
capture display abs(7 ! 4)
display "@ k1057 " _rc
capture display abs(3 + * 4)
display "@ k1058 " _rc
capture display abs(,)
display "@ k1059 " _rc
capture display abs()
display "@ k1060 " _rc
capture display x[1 @ 2]
display "@ k1061 " _rc
capture display x[1 2]
display "@ k1062 " _rc
capture scalar s = max(1 @ 2, 3)
display "@ k1063 " _rc
capture generate y = abs(x 2)
display "@ k1064 " _rc
generate xy = 7
capture display (1 e3)
display "@ k1065 " _rc
capture display (3 < = 4)
display "@ k1066 " _rc
capture display ("a b" + "c")
display "@ k1067 " _rc
capture display (x y)
display "@ k1068 " _rc
capture display (- - 3)
display "@ k1069 " _rc
capture display max(1 0 0, 5)
display "@ k1070 " _rc
capture display (2 . 5)
display "@ k1071 " _rc
capture display (x [1])
display "@ k1072 " _rc
capture display [1 2]
display "@ k1073 " _rc
capture display (1 2) + 1 2
display "@ k1074 " _rc
capture generate z = (x y)
display "@ k1075 " _rc
capture generate w = abs(x 2)
display "@ k1076 " _rc
capture display td(1 jan 2020)
display "@ k1077 " _rc
capture display tc(1jan2020 10:00)
display "@ k1078 " _rc
capture display string(1 2)
display "@ k1079 " _rc
capture display (3 4
display "@ k1080 " _rc
capture display ")" ((1)
display "@ k1081 " _rc
capture display 1 2)
display "@ k1082 " _rc
* what a rejected line leaves: macros, scalars and variables stay as they were, and a name it would have made is free
clear
set obs 3
generate x = _n
local m = 7
capture local m = 3 +
display "@ lm1 " _rc " `m'"
capture local m = 3 4
display "@ lm2 " _rc " `m'"
capture local m2 = (3
display "@ lm3 " _rc " [`m2']"
capture global gm = 4 \ 5
display "@ gm " _rc " [$gm]"
scalar s0 = 5
capture scalar s0 = 4 \ 5
display "@ sc " _rc " " s0
generate z = 1
capture replace z = z +
display "@ rz " _rc " " z[1]
capture generate w = 5 -
display "@ gw " _rc
capture confirm variable w
display "@ cw " _rc
generate w = 2
display "@ w " w[1] " " _N
capture assert x >
display "@ as " _rc
capture count if x 1
display "@ cn " _rc
capture display abs()
display "@ ab " _rc
capture display sqrt()
display "@ sq " _rc
capture display max()
display "@ mx " _rc
* a [ after a blank is a weight; list's if is read as an expression too
capture generate y = x [2]
display "@ wt1 " _rc
capture generate y = x [1]
display "@ wt2 " _rc
capture generate y = x [fweight=2]
display "@ wt3 " _rc
capture confirm variable y
display "@ wt4 " _rc
capture list x if +x in 1/1
display "@ ls1 " _rc
capture list x if x >
display "@ ls2 " _rc
capture list x if (x
display "@ ls3 " _rc
* a + sign goes only before a number written out
scalar sc = 2
capture display +x
display "@ p1 " _rc
capture display +(2)
display "@ p2 " _rc
capture display +abs(2)
display "@ p3 " _rc
capture display 1 + +x
display "@ p4 " _rc
capture display +5
display "@ p5 " _rc
capture display +sc
display "@ p6 " _rc
capture display +.a
display "@ p7 " _rc
capture generate g = +x
display "@ p8 " _rc
capture generate g = 1 + +x
display "@ p9 " _rc
capture generate g = x^+2
display "@ p10 " _rc
capture count if +x
display "@ p11 " _rc
capture count if x == +2
display "@ p12 " r(N)
capture scalar t = +sc
display "@ p13 " _rc
capture scalar t = +(2)
display "@ p14 " _rc
capture local t = +x
display "@ p15 " _rc
capture replace x = +x
display "@ p16 " _rc
capture assert +x
display "@ p17 " _rc
capture assert 1 + +2 == 3
display "@ p18 " _rc
