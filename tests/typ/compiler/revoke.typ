// Test revoke rules.

---
#show link: set text(blue)
#let plain(body) = {
  revoke link
  body
}

https://example.com
#plain[https://unstyled.org]

---
#set heading(numbering: "1.")
#show heading: it => "[" + it.body + "]"
= Hello

#revoke heading
= World

---
#show <mylabel>: set text(blue)
#let t = [*Text* <mylabel>]
#t
#revoke <mylabel>
#t

---
#show strong: set text(blue)
*Blue*
#[
  #revoke strong
  *Black*
  #show strong: set text(red)
  *Red*
]
*Blue*

---
#show raw: set text(red)
#show raw.where(block: true): revoke raw

This should be `red`.

```
This should be black.
```

---
#show raw.where(block: false): set text(red)
#revoke raw

This should be `black`.

```
This as well.
```

---
#show "hi!": "👋"
#show raw: revoke text

hey there! hi!
```rust
hi!("there");
```

---
#show regex("\d{2,3}"): box.with(
  stroke: 1pt,
  inset: (x: 2pt),
  outset: (y: 2pt),
)
My 123 number.

#revoke regex("\d{2,3}")
My 123 number.
