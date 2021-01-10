//! The versioning strategy, see [`Version`] for details

/// The versioning strategy
///
pub unsafe trait Version: Copy {
    /// Represents a full version
    type Save: Copy;

    /// The initial empty version
    const EMPTY: Self;

    /// Convert an full version to a empty version
    ///
    /// returns `None` if there are no more versions
    ///
    /// # Safety
    ///
    /// `mark_empty` can only be called on a full version
    unsafe fn mark_empty(self) -> Option<Self>;

    /// Convert an empty version to a full version
    ///
    /// # Safety
    ///
    /// `mark_full` can only be called on a empty version
    unsafe fn mark_full(self) -> Self;

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

    const EMPTY: Self = Self(0);

    unsafe fn mark_empty(self) -> Option<Self> { self.0.checked_add(1).map(Self) }

    unsafe fn mark_full(self) -> Self { Self(self.0 | 1) }

    fn is_full(self) -> bool { self.0 & 1 != 0 }

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

    const EMPTY: Self = Self(0);

    unsafe fn mark_empty(self) -> Option<Self> { self.0.checked_add(1).map(Self) }

    unsafe fn mark_full(self) -> Self { Self(self.0 | 1) }

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

    unsafe fn mark_empty(self) -> Option<Self> { Some(Self::Empty) }

    unsafe fn mark_full(self) -> Self { Self::Full }

    fn is_full(self) -> bool { matches!(self, Self::Full) }

    unsafe fn save(self) -> Self::Save { UnversionedFull(()) }

    fn equals_saved(self, UnversionedFull(()): Self::Save) -> bool { self.is_full() }
}
