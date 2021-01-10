# v 0.5.0

* Extract the core into `pui-core`
  * Re-exported as `pui::core`
* Add `pui::arena`
* Add `pui::vec`
* Add `pui::cell`

# v 0.4.0

* Removed `&mut I: Identifier`
* Adjusted `Identifier` safety docs 
  * `Identifier`s must *always* be unique on every thread they can be accessed on
    *even after they are dropped*
  * This allows them to be usd for safely used for unchecked indexing
* Fix a potential soundness hole in `make_typeid` and `make_typeid_tl`

# v 0.3.0

* Renamed `Counter` to `IdAlloc`
  * Allowed `IdAlloc` to be non-global
* Made `Trivial` safe to implement

# v 0.2.0

* Adjusted docs for `trait Identifier`
* Added `trait Handle` to better express the invariants

# v 0.1.1

* Added `trait Trivial` for marking handles which are trivial to construct, and have no validity or safety invariants
* Added a feature gate for atomic ops
* Removed the `Default` bound for `impl<T, U: PoolMut<T>> Pool<T> for Mutex<U>`
  * This was a oversight from copypasta
* made all trivial functions (one line that just constructs something, or calls something) `#[inline]`
* made `typeid` wait for a new typeid when building with `std`

# v 0.1.0 - Initial Release
