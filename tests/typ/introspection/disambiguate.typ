// Test location disambiguation.

---
#let v = [= Heading]
#v
#v
#locate(loc => query(heading, loc).map(it => it.location().position().y))

---
#set heading(numbering: "1.")
#let v = [= Heading]
#v
#v

---
#show figure: it => it.counter.display()
#figure(none)
#block(figure(none))
#figure(none)

---
#let c = counter("ok")
#let v = c.step() + c.display()
#v #v #v
#stack(dir: ltr, spacing: 4pt, v, v, v, v)

---
#set page(height: auto, width: auto)

#let q = counter("question")
#let step-show = {
    q.step()
    q.display("1")
}

#grid(step-show, step-show)

#pagebreak()
#step-show

#q.update(12)
#grid(step-show, step-show)
