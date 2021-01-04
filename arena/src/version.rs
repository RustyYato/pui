pub unsafe trait Version: Copy {
    type Save: Copy;

    const EMPTY: Self;

    unsafe fn mark_empty(self) -> Option<Self>;
    unsafe fn mark_full(self) -> Self;

    fn is_empty(self) -> bool { !self.is_full() }
    fn is_full(self) -> bool;

    unsafe fn save(self) -> Self::Save;
    fn equals_saved(self, saved: Self::Save) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefaultVersion(u32);

unsafe impl Version for DefaultVersion {
    type Save = Self;

    const EMPTY: Self = Self(0);

    unsafe fn mark_empty(self) -> Option<Self> { self.0.checked_add(1).map(Self) }

    unsafe fn mark_full(self) -> Self { Self(self.0 | 1) }

    fn is_full(self) -> bool { self.0 & 1 != 0 }

    unsafe fn save(self) -> Self::Save { self }

    fn equals_saved(self, saved: Self::Save) -> bool { self.0 == saved.0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TinyVersion(u8);

unsafe impl Version for TinyVersion {
    type Save = Self;

    const EMPTY: Self = Self(0);

    unsafe fn mark_empty(self) -> Option<Self> { self.0.checked_add(1).map(Self) }

    unsafe fn mark_full(self) -> Self { Self(self.0 | 1) }

    fn is_full(self) -> bool { self.0 & 1 != 0 }

    unsafe fn save(self) -> Self::Save { self }

    fn equals_saved(self, saved: Self::Save) -> bool { self.0 == saved.0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unversioned {
    Empty,
    Full,
}

unsafe impl Version for Unversioned {
    type Save = ();

    const EMPTY: Self = Self::Empty;

    unsafe fn mark_empty(self) -> Option<Self> { Some(Self::Empty) }

    unsafe fn mark_full(self) -> Self { Self::Full }

    fn is_full(self) -> bool { matches!(self, Self::Full) }

    unsafe fn save(self) -> Self::Save {}

    fn equals_saved(self, (): Self::Save) -> bool { self.is_full() }
}
