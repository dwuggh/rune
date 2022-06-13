//! This module holds implementation of conversion functions. Much of
//! this code could be replaced with macros or specialized generics if
//! those are ever stabalized.

use std::cell::RefCell;

use super::GcObj;
use super::{
    super::{
        cons::Cons,
        env::{sym, Symbol},
        error::{Error, Type},
    },
    HashTable,
};
use anyhow::Context;

use super::{Callable, Gc, Object};

#[allow(clippy::multiple_inherent_impl)]
impl Cons {
    pub(crate) fn try_as_macro(&self) -> anyhow::Result<Gc<Callable>> {
        match self.car().get() {
            Object::Symbol(sym) if sym == &sym::MACRO => {
                let cdr = self.cdr();
                let x = cdr.try_into()?;
                Ok(x)
            }
            x => Err(Error::from_object(Type::Symbol, x).into()),
        }
    }
}

impl<'ob> TryFrom<GcObj<'ob>> for usize {
    type Error = anyhow::Error;
    fn try_from(obj: GcObj<'ob>) -> Result<Self, Self::Error> {
        match obj.get() {
            Object::Int(x) => x
                .try_into()
                .with_context(|| format!("Integer must be positive, but was {}", x)),
            x => Err(Error::from_object(Type::Int, x).into()),
        }
    }
}

impl<'ob> TryFrom<GcObj<'ob>> for Option<usize> {
    type Error = anyhow::Error;
    fn try_from(obj: GcObj<'ob>) -> Result<Self, Self::Error> {
        match obj.get() {
            Object::Int(x) => match x.try_into() {
                Ok(x) => Ok(Some(x)),
                Err(e) => {
                    Err(e).with_context(|| format!("Integer must be positive, but was {}", x))
                }
            },
            Object::Nil => Ok(None),
            _ => Err(Error::from_object(Type::Int, obj).into()),
        }
    }
}

impl<'ob> TryFrom<GcObj<'ob>> for bool {
    type Error = Error;
    fn try_from(obj: GcObj) -> Result<Self, Self::Error> {
        match obj.get() {
            Object::Nil => Ok(false),
            _ => Ok(true),
        }
    }
}

impl<'ob> TryFrom<GcObj<'ob>> for Option<()> {
    type Error = Error;
    fn try_from(obj: GcObj) -> Result<Self, Self::Error> {
        match obj.get() {
            Object::Nil => Ok(None),
            _ => Ok(Some(())),
        }
    }
}

/// This function is required because we have no specialization yet.
/// Essentially this let's us convert one type to another "in place"
/// without the need to allocate a new slice. We ensure that the two
/// types have the exact same representation, so that no writes
/// actually need to be performed.
pub(crate) fn try_from_slice<'brw, 'ob, T, E>(slice: &'brw [GcObj<'ob>]) -> Result<&'brw [Gc<T>], E>
where
    Gc<T>: TryFrom<GcObj<'ob>, Error = E> + 'ob,
{
    for x in slice.iter() {
        let _new = Gc::<T>::try_from(*x)?;
    }
    let ptr = slice.as_ptr().cast::<Gc<T>>();
    let len = slice.len();
    Ok(unsafe { std::slice::from_raw_parts(ptr, len) })
}

impl<'ob> From<bool> for GcObj<'ob> {
    fn from(b: bool) -> Self {
        if b {
            GcObj::TRUE
        } else {
            GcObj::NIL
        }
    }
}

define_unbox!(Int, i64);
define_unbox!(Float, &'ob f64);
define_unbox!(String, &'ob String);
define_unbox!(String, &'ob str);
define_unbox!(Vec, &'ob RefCell<Vec<GcObj<'ob>>>);
define_unbox!(HashTable, &'ob RefCell<HashTable<'ob>>);
define_unbox!(Symbol, Symbol);

impl<'ob, T> From<Option<T>> for GcObj<'ob>
where
    T: Into<GcObj<'ob>>,
{
    fn from(t: Option<T>) -> Self {
        match t {
            Some(x) => x.into(),
            None => GcObj::NIL,
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::super::arena::{Arena, RootSet};

    use super::*;

    fn wrapper(args: &[GcObj]) -> Result<i64, Error> {
        Ok(inner(
            std::convert::TryFrom::try_from(args[0])?,
            std::convert::TryFrom::try_from(args[1])?,
        ))
    }

    fn inner(arg0: Option<i64>, arg1: &Cons) -> i64 {
        let x: i64 = arg1.car().try_into().unwrap();
        arg0.unwrap() + x
    }

    #[test]
    fn test() {
        let roots = &RootSet::default();
        let arena = &Arena::new(roots);
        let obj0 = arena.add(5);
        // SAFETY: We don't call garbage collect so references are valid
        let obj1 = unsafe { arena.add(Cons::new(1.into(), 2.into())) };
        let vec = vec![obj0, obj1];
        let res = wrapper(vec.as_slice());
        assert_eq!(6, res.unwrap());
    }
}