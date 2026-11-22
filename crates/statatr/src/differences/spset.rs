Guide {
    command: "spset",
    about: "Declare spatial data: the id variable and planar coordinates",
    points: &[
        p(
            "As Stata 19",
            "spset id, coord(x y) makes _ID (a copy of id, labelled Spatial-unit ID), _CX and \
             _CY and prints Stata's block; spset alone shows it again and spset, modify coord() \
             changes the coordinates. Stata's errors keep their return codes: data not spset, \
             already spset, option modify required, an id that does not identify the rows, \
             too few coordinate variables, no dataset in use.",
        ),
        p(
            "Differences",
            "Shapefiles, latitude and longitude (coordsys(latlong)), spset, clear and an id \
             with missing values are stopped, saying so.",
        ),
    ],
}
