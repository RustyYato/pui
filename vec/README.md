# pui-vec

A append-only vector that uses `pui_core` to brand indicies
to allow for unchecked indexing. (Note: `PuiVec` is only
append-only if there is an associated `Identifier` attached)

## Features

`pui` (default) - this hooks into `pui_core` and provides a
branded [`Id`] that can be used to elide bound checks.


License: MIT/Apache-2.0
