---
trigger: glob
globs: **/*.rs
---

Never use nested `if`'s with `if let`. The code base uses a rust version which allows to chain `if let` with `&&` and `||`.