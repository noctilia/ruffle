use gc_arena::{Collect, Gc, MutationContext};
use std::ops::Deref;

#[derive(Debug, Clone, Collect)]
#[collect(no_drop)]
enum Source<'gc> {
    Owned(Gc<'gc, String>),
    Static(&'static str),
}

#[derive(Debug, Clone, Collect)]
#[collect(no_drop)]
pub struct Avm1String<'gc> {
    source: Source<'gc>,
}

impl<'gc> Avm1String<'gc> {
    pub fn new<S: Into<String>>(gc_context: MutationContext<'gc, '_>, string: S) -> Self {
        Self {
            source: Source::Owned(Gc::allocate(gc_context, string.into())),
        }
    }

    pub fn as_str(&self) -> &str {
        self
    }
}

impl<'gc> From<&'static str> for Avm1String<'gc> {
    fn from(str: &'static str) -> Self {
        Self {
            source: Source::Static(str),
        }
    }
}

impl Deref for Avm1String<'_> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        match &self.source {
            Source::Owned(str) => str.deref(),
            Source::Static(str) => str,
        }
    }
}

impl AsRef<str> for Avm1String<'_> {
    #[inline]
    fn as_ref(&self) -> &str {
        match &self.source {
            Source::Owned(str) => str,
            Source::Static(str) => str,
        }
    }
}

macro_rules! impl_eq {
    ($lhs:ty, $rhs: ty) => {
        #[allow(unused_lifetimes)]
        impl<'a, 'b> PartialEq<$rhs> for $lhs {
            #[inline]
            fn eq(&self, other: &$rhs) -> bool {
                PartialEq::eq(&self[..], &other[..])
            }
        }

        #[allow(unused_lifetimes)]
        impl<'a, 'b> PartialEq<$lhs> for $rhs {
            #[inline]
            fn eq(&self, other: &$lhs) -> bool {
                PartialEq::eq(&self[..], &other[..])
            }
        }
    };
}

impl_eq! { Avm1String<'_>, str }
impl_eq! { Avm1String<'_>, &'a str }
impl_eq! { Avm1String<'_>, String }
