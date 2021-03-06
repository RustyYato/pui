use core::marker::PhantomData;

use crate::version::Version;

/// A key into a sparse arena
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct Key<Id, V = crate::version::SavedDefaultVersion> {
    id: Id,
    version: V,
}

/// An index validator
pub struct Validator<'a>(PhantomData<fn() -> *mut &'a ()>);
/// A completed index validator
pub struct CompleteValidator<'a>(bool, Validator<'a>);

impl<Id, V> Key<Id, V> {
    /// Create a new key from an id and version
    pub const fn new(id: Id, version: V) -> Self { Self { id, version } }

    /// The id the given key
    pub const fn id(&self) -> &Id { &self.id }

    /// The version of the given key
    pub const fn version(&self) -> &V { &self.version }
}

impl<'a> Validator<'a> {
    pub(crate) fn new() -> Self { Self(PhantomData) }

    /// complete this index validator with an unchecked index
    ///
    /// # Safety
    ///
    /// See `ArenaKey::validate_ident`
    #[allow(unused_variables)]
    pub unsafe fn unchecked_index<I>(self, ident: &'a I) -> CompleteValidator<'a> { CompleteValidator(true, self) }

    /// complete this index validator with an index
    pub fn checked_index(self) -> CompleteValidator<'a> { CompleteValidator(false, self) }
}

impl CompleteValidator<'_> {
    pub(crate) fn into_inner(self) -> bool { self.0 }
}

/// A trait to access elements of an `Arena`
pub trait ArenaKey<I, V: Version> {
    /// An optimization that allows you to construct an unchecked index into the `Arena`
    ///
    /// It is only safe to call [`Validator::unchecked_index`]
    /// if the next call to `Self::index` is guarnteed to be in bounds
    /// for the arena with the identifier `ident`
    #[allow(unused_variables)]
    fn validate_ident<'a>(&self, ident: &'a I, validator: Validator<'a>) -> CompleteValidator<'a> {
        validator.checked_index()
    }

    /// The index of this key
    fn index(&self) -> usize;

    /// The version of this key
    fn version(&self) -> Option<V::Save>;
}

/// A trait to create keys from an arena
pub trait BuildArenaKey<I, V: Version>: ArenaKey<I, V> {
    /// Create a new arena key given an index, version save, and identifier
    ///
    /// # Safety
    ///
    /// * `index` must be in bounds of the arena with the identifier `ident`
    /// * `save` must be the latest save of slot at `index` in the arena
    unsafe fn new_unchecked(index: usize, save: V::Save, ident: &I) -> Self;
}

impl<K: ?Sized + ArenaKey<I, V>, I, V: Version> ArenaKey<I, V> for &K {
    fn validate_ident<'a>(&self, ident: &'a I, validator: Validator<'a>) -> CompleteValidator<'a> {
        K::validate_ident(self, ident, validator)
    }

    fn index(&self) -> usize { K::index(self) }

    fn version(&self) -> Option<V::Save> { K::version(self) }
}

impl<I, V: Version> ArenaKey<I, V> for usize {
    fn index(&self) -> usize { *self }

    fn version(&self) -> Option<V::Save> { None }
}

impl<I, V: Version> BuildArenaKey<I, V> for usize {
    #[doc(hidden)]
    unsafe fn new_unchecked(index: usize, _: V::Save, _: &I) -> Self { index }
}

impl<I, V: Version> ArenaKey<I, V> for crate::TrustedIndex {
    fn validate_ident<'a>(&self, ident: &'a I, validator: Validator<'a>) -> CompleteValidator<'a> {
        unsafe { validator.unchecked_index(ident) }
    }

    fn index(&self) -> usize { self.0 }

    fn version(&self) -> Option<V::Save> { None }
}

#[cfg(feature = "pui-core")]
#[cfg_attr(docsrs, doc(cfg(feature = "pui")))]
impl<I: pui_core::OneShotIdentifier, V: Version> ArenaKey<I, V> for pui_vec::Id<I::Token> {
    fn validate_ident<'a>(&self, ident: &'a I, validator: Validator<'a>) -> CompleteValidator<'a> {
        if ident.owns_token(self.token()) {
            unsafe { validator.unchecked_index(ident) }
        } else {
            validator.checked_index()
        }
    }

    fn index(&self) -> usize { self.get() }

    fn version(&self) -> Option<V::Save> { None }
}

#[cfg(feature = "pui-core")]
#[cfg_attr(docsrs, doc(cfg(feature = "pui")))]
impl<I: pui_core::OneShotIdentifier, V: Version> BuildArenaKey<I, V> for pui_vec::Id<I::Token> {
    #[doc(hidden)]
    unsafe fn new_unchecked(index: usize, _: V::Save, ident: &I) -> Self {
        pui_vec::Id::new_unchecked(index, ident.token())
    }
}

impl<I, V: Version> ArenaKey<I, V> for Key<usize, V::Save> {
    fn index(&self) -> usize { self.id }

    fn version(&self) -> Option<V::Save> { Some(self.version) }
}

impl<I, V: Version> BuildArenaKey<I, V> for Key<usize, V::Save> {
    #[doc(hidden)]
    unsafe fn new_unchecked(index: usize, version: V::Save, _: &I) -> Self { Key { id: index, version } }
}

#[cfg(feature = "pui-core")]
#[cfg_attr(docsrs, doc(cfg(feature = "pui")))]
impl<I: pui_core::OneShotIdentifier, V: Version> ArenaKey<I, V> for Key<pui_vec::Id<I::Token>, V::Save> {
    fn validate_ident<'a>(&self, ident: &'a I, validator: Validator<'a>) -> CompleteValidator<'a> {
        if ident.owns_token(self.id().token()) {
            unsafe { validator.unchecked_index(ident) }
        } else {
            validator.checked_index()
        }
    }

    fn index(&self) -> usize { self.id.get() }

    fn version(&self) -> Option<V::Save> { Some(self.version) }
}

#[cfg(feature = "pui-core")]
#[cfg_attr(docsrs, doc(cfg(feature = "pui")))]
impl<I: pui_core::OneShotIdentifier, V: Version> BuildArenaKey<I, V> for Key<pui_vec::Id<I::Token>, V::Save> {
    #[doc(hidden)]
    unsafe fn new_unchecked(index: usize, version: V::Save, ident: &I) -> Self {
        Key {
            id: pui_vec::Id::new_unchecked(index, ident.token()),
            version,
        }
    }
}

impl<I, V: Version> ArenaKey<I, V> for Key<crate::TrustedIndex, V::Save> {
    fn validate_ident<'a>(&self, ident: &'a I, validator: Validator<'a>) -> CompleteValidator<'a> {
        unsafe { validator.unchecked_index(ident) }
    }

    fn index(&self) -> usize { self.id.0 }

    fn version(&self) -> Option<V::Save> { Some(self.version) }
}
