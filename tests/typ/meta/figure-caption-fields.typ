// Test accessing figure caption fields in figure show rule.

---
// Test normal figure caption set rule.
#set figure.caption(separator: " <> ")
#figure(caption: [Caption])[Content]

---
// Test show-set rules on figure caption.
#show figure.caption: set text(red)
#show figure.caption.where(body: [Caption I]): set figure.caption(separator: " <> ")

#figure(caption: [Caption I])[Content]
#figure(caption: [Caption II])[Content]

// Test accessing field on caption in figure show rule.
#show figure: it => it.caption.separator
#figure(caption: [Caption I])[Content]
#figure(caption: [Caption II])[Content]

---
// Test it again.
#show figure: it => it.caption.separator
#figure(caption: [Caption])[Content]

---
// This does *not* work because the caption is already materialized in the
// figure show rule. This is similar to how `set figure(..)` itself doesn't work
// in a figure show rule.
#show figure: it => {
  set figure.caption(separator: " <> ")
  it
}

#figure(caption: [Caption])[Content]
