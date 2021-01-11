# pui

This repository contains **p**rocess **u**nique **i**dentifiers (PUIs) and abstractions built atop them.

* `pui` - is a facade crate that just re-exports the other crates for ease of use
* `pui-core` - contains the core primitives requried to support PUIs and a few implementations of PUIs
* `pui-cell` - builds atop `pui-core` to provide thread-safe shared mutable cells that don't require guards
* `pui-vec` - provides an append only vector that can elide bounds checks
* `pui-arena` - builds on `pui-vec` to provide generalized arenas