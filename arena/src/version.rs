//! The versioning strategy, see [`Version`] for details

/// The versioning strategy
///
/// # Slot Exhaustion
///
/// Arenas store a version alongside each slot they have. If a slot's version is
/// exhausted, then that slot will *never* be reused. For [`DefaultVersion`]
/// this will only happen after about 2 billion insertion/deletion pairs *per slot*,
/// so it shouldn't be an issue. However, for smaller version types like [`TinyVersion`]
/// each slot will exhaust after only 127 insertion/deletion pairs per slot.
///
/// You can avoid version exahustion by using [`Unversioned`], but this suffers from the
/// ABA problem.
///
/// # ABA problem
///
/// [Wikipedia](https://en.wikipedia.org/wiki/ABA_problem)
///
/// Consider this sequence of operations:
///
/// ``` should_panic
/// use pui_arena::version::UnversionedFull;
/// # use pui_arena::Key;
/// # let mut arena = pui_arena::base::sparse::Arena::<_, (), pui_arena::version::Unversioned>::INIT;
/// // insert 0 at index 0, arena = [(version: Unversioned::Full, Some(0))]
/// let a: Key<usize, UnversionedFull> = arena.insert(0);
/// // remove at index 0, arena = [(version: Unversioned::Empty, None)]
/// arena.remove(a);
/// // insert 10 at index 0, arena = [(version: Unversioned::Full, Some(10))]
/// let b: Key<usize, UnversionedFull> = arena.insert(10);
/// // get the value at index 0
/// assert_eq!(arena.get(a), None);
/// ```
///
/// because `arena` is using `Unversioned` slots, even though key `a` was removed from the
/// `arena`, we can still access it! This is fundementally what the ABA problem is. Even
/// though a key was removed, the arena can't distinguish stale keys so it allows access
/// to the new values when it shouldn't.
///
/// Depending on your problem space, this may or may not be a problem you need to sove.
/// If it is a problem you need to solve, then you will need to deal with version exhaustion
/// in some way, because there are never going to be an infinite number of versions. `pui-arena`
/// handles this for you by "leaking" slots with exhausted versions. These slots will not
/// be reused, but will be deallocated once the `Arena` drops.
pub unsafe trait Version: Copy {
    /// Represents a full version
    type Save: Copy;

    /// The initial empty version
    const EMPTY: Self;

    /// Convert an full version to a empty version
    ///
    /// returns `Err` if there are no more versions
    ///
    /// # Safety
    ///
    /// `mark_empty` can only be called on a full version
    unsafe fn mark_empty(self) -> Result<Self, Self>;

    /// Convert an empty version to a full version
    ///
    /// # Safety
    ///
    /// `mark_full` can only be called on a empty version
    unsafe fn mark_full(self) -> Self;

    /// Check if the version is exhausted
    fn is_exhausted(&self) -> bool;
    /// Check if the version is empty
    fn is_empty(self) -> bool { !self.is_full() }
    /// Check if the version is full
    fn is_full(self) -> bool;

    /// Save the current version
    ///
    /// # Safety
    ///
    /// `save` can only be called on a full version
    unsafe fn save(self) -> Self::Save;

    /// Check if the saved version matches the current version
    ///
    /// In particular, this can only be true if the current version is full
    /// and may not be true if there was a call to `mark_empty` in since the
    /// save was created.
    fn equals_saved(self, saved: Self::Save) -> bool;
}

/// The default versioning strategy, that's backed by a [`u32`], that avoids the
/// [`ABA problem`](https://en.wikipedia.org/wiki/ABA_problem)
///
/// This can track up to 2^31 insertion-deletion pairs before exhaustion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefaultVersion(u32);
/// `<DefaultVersion as Version>::Save`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedDefaultVersion(u32);

unsafe impl Version for DefaultVersion {
    type Save = SavedDefaultVersion;

    const EMPTY: Self = Self(1);

    unsafe fn mark_empty(self) -> Result<Self, Self> {
        let next = Self(self.0 | 1);
        match self.0.checked_add(2) {
            Some(_) => Ok(next),
            None => Err(next),
        }
    }

    fn is_exhausted(&self) -> bool { self.0 == u32::MAX }

    unsafe fn mark_full(self) -> Self { Self(self.0.wrapping_add(1)) }

    fn is_full(self) -> bool { self.0 & 1 == 0 }

    unsafe fn save(self) -> Self::Save { SavedDefaultVersion(self.0) }

    fn equals_saved(self, saved: Self::Save) -> bool { self.0 == saved.0 }
}

/// A small versioning strategy, that's backed by a [`u8`], that avoids the
/// [`ABA problem`](https://en.wikipedia.org/wiki/ABA_problem)
///
/// This can track up to 2^7 insertion-deletion pairs before exhaustion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TinyVersion(u8);
/// `<TinyVersion as Version>::Save`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedTinyVersion(u8);

unsafe impl Version for TinyVersion {
    type Save = SavedTinyVersion;

    const EMPTY: Self = Self(1);

    unsafe fn mark_empty(self) -> Result<Self, Self> {
        let next = Self(self.0 | 1);
        match self.0.checked_add(2) {
            Some(_) => Ok(next),
            None => Err(next),
        }
    }

    unsafe fn mark_full(self) -> Self { Self(self.0.wrapping_add(1)) }

    fn is_exhausted(&self) -> bool { self.0 == u8::MAX }

    fn is_full(self) -> bool { self.0 & 1 != 0 }

    unsafe fn save(self) -> Self::Save { SavedTinyVersion(self.0) }

    fn equals_saved(self, saved: Self::Save) -> bool { self.0 == saved.0 }
}

/// A versioning strategy that doesn't actually track versions,
/// just the state of the container. This strategy can fall prey
/// to the [`ABA problem`](https://en.wikipedia.org/wiki/ABA_problem)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unversioned {
    /// The contianer is empty
    Empty,
    /// The contianer is full
    Full,
}
/// `<UnversionedFull as Version>::Save`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnversionedFull(());

unsafe impl Version for Unversioned {
    type Save = UnversionedFull;

    const EMPTY: Self = Self::Empty;

    unsafe fn mark_empty(self) -> Result<Self, Self> { Ok(Self::Empty) }

    unsafe fn mark_full(self) -> Self { Self::Full }

    fn is_exhausted(&self) -> bool { false }

    fn is_full(self) -> bool { matches!(self, Self::Full) }

    unsafe fn save(self) -> Self::Save { UnversionedFull(()) }

    fn equals_saved(self, UnversionedFull(()): Self::Save) -> bool { self.is_full() }
}
