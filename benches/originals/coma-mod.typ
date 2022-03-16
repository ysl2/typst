#set page(margins: 2.25cm)

*Technische Universität Berlin* #h(1fr) *WiSe 2019/2020* \
*Fakultät II, Institut for Mathematik* #h(1fr) Woche 3 \
Sekretariat MA \
Dr. Max Mustermann \
Ola Nordmann, John Doe

#v(2mm)
#align(center)[
  ==== 3. Übungsblatt Computerorientierte Mathematik II #v(1mm)
  *Abgabe: 03.05.2019* (bis 10:10 Uhr in MA 001) #v(1mm)
  *Alle Antworten sind zu beweisen.*
]

#let aufgabe(n, p) = block[
  #v(6pt)
  *#n;. Aufgabe* #h(1fr) (#p Punkte)
]

#aufgabe(1)[1 + 1 + 2]

Ein _Binärbaum_ ist ein Wurzelbaum, in dem jeder Knoten ≤ 2 Kinder hat.
Die Tiefe eines Knotens _v_ ist die Länge des eindeutigen Weges von der Wurzel
zu _v_, und die Höhe von _v_ ist die Länge eines längsten (absteigenden) Weges
von _v_ zu einem Blatt. Die Höhe des Baumes ist die Höhe der Wurzel.

#align(center, image("graph.png", width: 75%))

Berechnen Sie, wie viele Kanten maximal begangen werden müssen, um in einem Wurzelbaum mit der Tiefe _v_ von einem gegebenen Knoten jeden anderen Knoten zu erreichen. Beweisen Sie Ihr Ergebnis nach dem in der Vorlesung besprochenen Schema.

#aufgabe(2)[2 + 1 + 5]

Ein _primes_ Element eines Zahlenkörpers teilt das Produkt _ab_ genau dann, wenn es entweder _a_ oder _b_ teilt.

1. Beschreiben Sie, wie sich ein primes Element und eine Primzahl unterscheiden? (2 Punkte)
2. Hat jeder Zahlenkörper, der Primzahlen enthält, auch prime Elemente? (1 Punkt)
3. Beweisen Sie, dass die Polynome $x^2 − 2$ und $x^2 + 1$ in *Z*\[x\], dem Ring der Polynome über *Z*, Primelemente sind.
