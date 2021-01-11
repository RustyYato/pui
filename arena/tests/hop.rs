use pui_arena::base::hop::Arena;

type Key = pui_arena::Key<usize, SavedTestVersion>;

#[derive(Debug, Clone, Copy)]
struct TestVersion(u8);
#[derive(Clone, Copy)]
struct SavedTestVersion(u8);

unsafe impl pui_arena::version::Version for TestVersion {
    type Save = SavedTestVersion;
    const EMPTY: Self = Self(0);

    unsafe fn mark_empty(self) -> Result<Self, Self> {
        if self.0 == 3 {
            Err(Self(u8::MAX - 1))
        } else {
            Ok(Self(self.0 + 1))
        }
    }

    unsafe fn mark_full(self) -> Self { Self(self.0 | 1) }

    fn is_exhausted(&self) -> bool { self.0 == u8::MAX - 1 }

    fn is_full(self) -> bool { self.0 & 1 != 0 }

    fn equals_saved(self, saved: Self::Save) -> bool { self.0 == saved.0 }

    unsafe fn save(self) -> Self::Save { SavedTestVersion(self.0) }
}

#[test]
fn hop_version_exhaustion() {
    let mut arena = Arena::<_, (), TestVersion>::with_ident(());
    let a: Key = arena.insert(0);
    let ai = *a.id();
    let a = arena.remove(a);
    let a: Key = arena.insert(a);
    let bi = *a.id();
    assert_eq!(ai, bi);
    let a = arena.remove(a);
    let a: Key = arena.insert(a);
    let ci = *a.id();
    assert_ne!(ai, ci);
    let a = arena.remove(a);
    let a: Key = arena.insert(a);
    let di = *a.id();
    assert_eq!(ci, di);
}
