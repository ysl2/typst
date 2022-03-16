// at middle
//%% INSERT
#grid(
  columns: (1fr, 3cm, 2fr, auto),
  gutter: 6pt,
  
  rect(width: 100%, padding: 6pt)[A rectangle with some text could be here.],
  [
    Lorem ipsum dolor sit amet,
    sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.
  ],
  rect(
    width: 100%,
    fill: maroon,
    stroke: black,
    padding: 8pt,
    align(center, circle(radius: 20pt, stroke: white, thickness: 3pt))
  ),
  rect(
    width: 5cm,
    padding: 6pt,
    for i in range(8) {rect(width: 20%, fill: rgb(100%, 100% * (i / 8), 0%))}
  ),
  [
    #set par(lang: "de")
    Meine Oma fährt im Hühnerstall Motorrad.
  ],
  rect(width: 100%),
  align(center)[*Vexillology*],
  align(bottom+right, square(width: 4pt, fill:black))
)
//%% END