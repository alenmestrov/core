use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;

use calimero_primitives::reflect::{DynReflect, Reflect, ReflectExt};

trait BufRef: Reflect {
    fn buf(&self) -> &[u8];
}

impl<'a, T: AsRef<[u8]> + 'a> BufRef for T {
    fn buf(&self) -> &[u8] {
        self.as_ref()
    }
}

impl<'a> fmt::Debug for dyn BufRef + Send + Sync + 'a {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.type_name())
    }
}

impl<'a> fmt::Debug for dyn BufRef + 'a {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.type_name())
    }
}

trait SliceBehavior<'a>: AsRef<[u8]> + Sized {
    type RefCountedObject<T: 'a>;

    fn from_boxed(inner: Box<[u8]>) -> Self;

    fn into_boxed(self) -> Box<[u8]>;
    fn owned_ref<B: AsRef<[u8]>>(&'a self) -> Option<&'a B>;
    fn take_owned<B: AsRef<[u8]> + 'a>(self) -> Result<Self::RefCountedObject<B>, Self>;
}

trait ThreadMode {
    type Inner<'a>: SliceBehavior<'a>;
}

enum SliceInner<'a, T: ThreadMode> {
    Raw(&'a [u8]),
    Ref(T::Inner<'a>),
}

impl<'a, T: ThreadMode<Inner<'a>: fmt::Debug>> fmt::Debug for SliceInner<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Raw(inner) => f.debug_tuple("Raw").field(inner).finish(),
            Self::Ref(inner) => f.debug_tuple("Ref").field(inner).finish(),
        }
    }
}

impl<'a, T: ThreadMode<Inner<'a>: Clone>> Clone for SliceInner<'a, T> {
    fn clone(&self) -> Self {
        match self {
            Self::Raw(inner) => Self::Raw(inner.clone()),
            Self::Ref(inner) => Self::Ref(inner.clone()),
        }
    }
}

#[derive(Clone, Debug)]
enum AtomicSlice<'a> {
    Box(Arc<Box<[u8]>>),
    Any(Arc<dyn BufRef + Send + Sync + 'a>),
}

impl<'a> AsRef<[u8]> for AtomicSlice<'a> {
    fn as_ref(&self) -> &[u8] {
        match self {
            AtomicSlice::Box(inner) => inner.as_ref(),
            AtomicSlice::Any(inner) => inner.buf(),
        }
    }
}

pub enum MultiThreaded {}

impl ThreadMode for MultiThreaded {
    type Inner<'a> = AtomicSlice<'a>;
}

impl<'a> SliceBehavior<'a> for AtomicSlice<'a> {
    type RefCountedObject<T: 'a> = Arc<T>;

    fn from_boxed(inner: Box<[u8]>) -> Self {
        AtomicSlice::Box(Arc::new(inner))
    }

    fn into_boxed(self) -> Box<[u8]> {
        let ref_boxed = match self {
            AtomicSlice::Box(inner) => inner,
            AtomicSlice::Any(inner) => match inner.with_arc(<dyn Reflect>::downcast_arc) {
                Ok(inner) => inner,
                Err(inner) => return inner.buf().into(),
            },
        };

        Arc::try_unwrap(ref_boxed).unwrap_or_else(|inner| (*inner).clone())
    }

    fn owned_ref<B: AsRef<[u8]>>(&'a self) -> Option<&'a B> {
        if let AtomicSlice::Any(inner) = self {
            if let Some(inner) = inner.as_dyn().downcast_ref::<B>() {
                return Some(inner);
            }
        }

        None
    }

    fn take_owned<B: AsRef<[u8]> + 'a>(self) -> Result<Self::RefCountedObject<B>, Self> {
        if let AtomicSlice::Any(inner) = self {
            return match inner.with_arc(<dyn Reflect>::downcast_arc) {
                Ok(inner) => Ok(inner),
                Err(inner) => Err(AtomicSlice::Any(inner)),
            };
        };

        Err(self)
    }
}

#[derive(Clone, Debug)]
enum RefCountedSlice<'a> {
    Box(Rc<Box<[u8]>>),
    Any(Rc<dyn BufRef + 'a>),
}

impl<'a> AsRef<[u8]> for RefCountedSlice<'a> {
    fn as_ref(&self) -> &[u8] {
        match self {
            RefCountedSlice::Box(inner) => inner.as_ref(),
            RefCountedSlice::Any(inner) => inner.buf(),
        }
    }
}

pub enum SingleThreaded {}

impl ThreadMode for SingleThreaded {
    type Inner<'a> = RefCountedSlice<'a>;
}

impl<'a> SliceBehavior<'a> for RefCountedSlice<'a> {
    type RefCountedObject<T: 'a> = Rc<T>;

    fn from_boxed(inner: Box<[u8]>) -> Self {
        RefCountedSlice::Box(Rc::new(inner))
    }

    fn into_boxed(self) -> Box<[u8]> {
        let ref_boxed = match self {
            RefCountedSlice::Box(inner) => inner,
            RefCountedSlice::Any(inner) => match inner.with_rc(<dyn Reflect>::downcast_rc) {
                Ok(inner) => inner,
                Err(inner) => return inner.buf().into(),
            },
        };

        Rc::try_unwrap(ref_boxed).unwrap_or_else(|inner| (*inner).clone())
    }

    fn owned_ref<B: AsRef<[u8]>>(&'a self) -> Option<&'a B> {
        if let RefCountedSlice::Any(inner) = self {
            if let Some(inner) = inner.as_dyn().downcast_ref::<B>() {
                return Some(inner);
            }
        }

        None
    }

    fn take_owned<B: AsRef<[u8]> + 'a>(self) -> Result<Self::RefCountedObject<B>, Self> {
        if let RefCountedSlice::Any(inner) = self {
            return match inner.with_rc(<dyn Reflect>::downcast_rc) {
                Ok(inner) => Ok(inner),
                Err(inner) => Err(RefCountedSlice::Any(inner)),
            };
        };

        Err(self)
    }
}

pub struct Slice<'a, T: ThreadMode = SingleThreaded> {
    inner: SliceInner<'a, T>,
}

pub type SingleThreadedSlice<'a> = Slice<'a, SingleThreaded>;
pub type MultiThreadedSlice<'a> = Slice<'a, MultiThreaded>;

impl<'a, T: ThreadMode<Inner<'a>: Clone>> Clone for Slice<'a, T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<'a> Slice<'a, MultiThreaded> {
    pub fn from_owned<T: AsRef<[u8]> + Send + Sync + 'a>(inner: T) -> Self {
        Self {
            inner: SliceInner::Ref(AtomicSlice::Any(Arc::new(inner) as _)),
        }
    }
}

impl<'a> Slice<'a, SingleThreaded> {
    pub fn from_owned<T: AsRef<[u8]> + 'a>(inner: T) -> Self {
        Self {
            inner: SliceInner::Ref(RefCountedSlice::Any(Rc::new(inner) as _)),
        }
    }
}

impl<'a, T: ThreadMode> Slice<'a, T> {
    pub fn into_boxed(self) -> Box<[u8]> {
        match self.inner {
            SliceInner::Raw(inner) => inner.into(),
            SliceInner::Ref(inner) => T::Inner::into_boxed(inner),
        }
    }

    pub fn owned_ref<B: AsRef<[u8]>>(&'a self) -> Option<&'a B> {
        if let SliceInner::Ref(inner) = &self.inner {
            return T::Inner::owned_ref(inner);
        }

        None
    }

    /// Take the inner value if it is of the correct type passed in via `from_owned`.
    pub fn take_owned<B: AsRef<[u8]> + 'a>(
        self,
    ) -> Result<<T::Inner<'a> as SliceBehavior<'a>>::RefCountedObject<B>, Self> {
        if let SliceInner::Ref(inner) = self.inner {
            match T::Inner::take_owned(inner) {
                Ok(owned) => return Ok(owned),
                Err(inner) => {
                    return Err(Self {
                        inner: SliceInner::Ref(inner),
                    })
                }
            }
        };

        Err(self)
    }
}

impl<'a, T: ThreadMode> Deref for Slice<'a, T> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'a, T: ThreadMode> AsRef<[u8]> for Slice<'a, T> {
    fn as_ref(&self) -> &[u8] {
        match &self.inner {
            SliceInner::Raw(inner) => inner,
            SliceInner::Ref(inner) => inner.as_ref(),
        }
    }
}

impl<'a, B: AsRef<[u8]> + ?Sized> From<&'a B> for Slice<'a, MultiThreaded> {
    fn from(inner: &'a B) -> Self {
        Self {
            inner: SliceInner::Raw(inner.as_ref()),
        }
    }
}

impl<'a> From<Box<[u8]>> for Slice<'a, MultiThreaded> {
    fn from(inner: Box<[u8]>) -> Self {
        Self {
            inner: SliceInner::Ref(AtomicSlice::from_boxed(inner)),
        }
    }
}

impl<'a> From<Vec<u8>> for Slice<'a, MultiThreaded> {
    fn from(inner: Vec<u8>) -> Self {
        inner.into_boxed_slice().into()
    }
}

impl<'a> From<Arc<Box<[u8]>>> for Slice<'a, MultiThreaded> {
    fn from(inner: Arc<Box<[u8]>>) -> Self {
        Self {
            inner: SliceInner::Ref(AtomicSlice::Box(inner)),
        }
    }
}

impl<'a> From<Rc<Box<[u8]>>> for Slice<'a, SingleThreaded> {
    fn from(inner: Rc<Box<[u8]>>) -> Self {
        Self {
            inner: SliceInner::Ref(RefCountedSlice::Box(inner)),
        }
    }
}

impl<'a, T: ThreadMode> From<Slice<'a, T>> for Box<[u8]> {
    fn from(slice: Slice<'a, T>) -> Self {
        slice.into_boxed()
    }
}

impl<'a, T: ThreadMode> Eq for Slice<'a, T> {}

impl<'a, 'b, A: ThreadMode, B: ThreadMode> PartialEq<Slice<'b, B>> for Slice<'a, A> {
    fn eq(&self, other: &Slice<'b, B>) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl<'a, T: ThreadMode> Ord for Slice<'a, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_ref().cmp(other.as_ref())
    }
}

impl<'a, 'b, A: ThreadMode, B: ThreadMode> PartialOrd<Slice<'b, B>> for Slice<'a, A> {
    fn partial_cmp(&self, other: &Slice<'b, B>) -> Option<std::cmp::Ordering> {
        self.as_ref().partial_cmp(other.as_ref())
    }
}

impl<'a, T: ThreadMode<Inner<'a>: fmt::Debug>> fmt::Debug for Slice<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("Slice").field(&self.inner).finish()
        } else {
            write!(f, "{:?}", self.as_ref())
        }
    }
}

impl<'a, T: ThreadMode> Borrow<[u8]> for Slice<'a, T> {
    fn borrow(&self) -> &[u8] {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slice_slice() {
        let data = b"hello";
        let slice = Slice::from(&data[..]);

        assert_eq!(slice.as_ref(), data);
        assert_eq!(&*slice.into_boxed(), data);
    }

    #[test]
    fn test_slice_vec() {
        let data = vec![0; 5];
        let slice = Slice::from(data);

        assert_eq!(slice.as_ref(), [0; 5]);
        assert_eq!(&*slice.into_boxed(), [0; 5]);
    }

    #[test]
    fn test_slice_box() {
        let data: Box<[u8]> = Box::new([0; 5]);
        let slice = Slice::from(data);

        assert_eq!(slice.as_ref(), [0; 5]);
        assert_eq!(&*slice.into_boxed(), [0; 5]);
    }

    #[test]
    fn test_slice_any() {
        struct Buf<'a>(&'a [u8]);

        impl AsRef<[u8]> for Buf<'_> {
            fn as_ref(&self) -> &[u8] {
                self.0
            }
        }

        let data = Buf(b"hello");
        let slice = SingleThreadedSlice::from_owned(data);

        assert_eq!(slice.as_ref(), b"hello");
        assert_eq!(&*slice.into_boxed(), b"hello");
    }

    #[test]
    fn test_owned_slice() {
        let data = b"hello";
        let slice = SingleThreadedSlice::from_owned(&data[..]);

        let slice = slice.take_owned::<[u8; 5]>().unwrap_err();
        let slice = slice.take_owned::<&[u8; 5]>().unwrap_err();
        let slice = slice.take_owned::<Vec<u8>>().unwrap_err();
        let slice = slice.take_owned::<Box<[u8]>>().unwrap_err();

        let slice = slice.take_owned::<&[u8]>().unwrap();

        assert_eq!(*slice, data);
    }

    #[test]
    fn test_owned_array() {
        let data = [0; 5];
        let slice = SingleThreadedSlice::from_owned(data);

        let slice = slice.take_owned::<&[u8]>().unwrap_err();
        let slice = slice.take_owned::<&[u8; 5]>().unwrap_err();
        let slice = slice.take_owned::<Vec<u8>>().unwrap_err();
        let slice = slice.take_owned::<Box<[u8]>>().unwrap_err();

        let slice = slice.take_owned::<[u8; 5]>().unwrap();

        assert_eq!(*slice, data);
    }

    #[test]
    fn test_owned_vec() {
        let data = vec![0; 5];
        let slice = SingleThreadedSlice::from_owned(data);

        let slice = slice.take_owned::<&[u8]>().unwrap_err();
        let slice = slice.take_owned::<&[u8; 5]>().unwrap_err();
        let slice = slice.take_owned::<[u8; 5]>().unwrap_err();
        let slice = slice.take_owned::<Box<[u8]>>().unwrap_err();

        let slice = slice.take_owned::<Vec<u8>>().unwrap();

        assert_eq!(*slice, [0; 5]);
    }

    #[test]
    fn test_owned_any() {
        struct Buf<'a>(&'a [u8]);

        impl AsRef<[u8]> for Buf<'_> {
            fn as_ref(&self) -> &[u8] {
                self.0
            }
        }

        let data = Buf(b"hello");
        let slice = SingleThreadedSlice::from_owned(data);

        let slice = slice.take_owned::<Buf>().unwrap();

        assert_eq!(slice.0, b"hello");
    }
}
