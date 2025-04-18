use super::{Gc, Object, ObjectType, TagType, WithLifetime};
use crate::{
    core::{
        error::{Type, TypeError},
        gc::{Block, Context, GcHeap, GcState, Trace},
    },
    derive_GcMoveable,
    intervals::IntervalTree,
};
use anyhow::{Result, bail};
use rune_macros::Trace;
use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
    sync::{Mutex, MutexGuard},
};
use text_buffer::Buffer as TextBuffer;

/// A Handle to an open buffer. Only one thread can hold this at a time.
#[derive(Debug)]
pub(crate) struct OpenBuffer<'a> {
    data: MutexGuard<'a, Option<BufferData>>,
    back_ref: &'a LispBuffer,
}

impl OpenBuffer<'_> {
    pub(crate) fn get(&self) -> &BufferData {
        // buffer can never be none because we check it as part of `lock`.
        self.data.as_ref().unwrap()
    }

    pub(crate) fn get_mut(&mut self) -> &mut BufferData {
        // buffer can never be none because we check it as part of `lock`.
        self.data.as_mut().unwrap()
    }

    // TODO: we shouldn't leave it empty
    pub(crate) fn kill(&mut self) -> bool {
        let killed = self.data.is_some();
        *self.data = None;
        killed
    }

    pub(crate) fn lisp_buffer<'ob>(&self, cx: &'ob Context) -> &'ob LispBuffer {
        cx.bind(self.back_ref)
    }

    pub(crate) fn insert(&mut self, arg: Object) -> Result<()> {
        match arg.untag() {
            ObjectType::Int(i) => {
                let Ok(u_32) = i.try_into() else { bail!("{i} is an invalid char") };
                let Some(chr) = char::from_u32(u_32) else { bail!("{i} is an Invalid char") };
                self.get_mut().text.insert_char(chr);
            }
            ObjectType::String(s) => self.get_mut().text.insert(s),
            x => bail!(TypeError::new(Type::String, x)),
        }
        Ok(())
    }

    pub(crate) fn slice_with_gap(&self, beg: usize, end: usize) -> Result<(&str, &str)> {
        let beg = self.in_range(beg)?;
        let end = self.in_range(end)?;
        Ok(self.get().text.slice(beg..end))
    }

    pub(crate) fn delete(&mut self, beg: usize, end: usize) -> Result<()> {
        let beg = self.in_range(beg)?;
        let end = self.in_range(end)?;
        self.get_mut().text.delete_range(beg, end);
        Ok(())
    }

    fn in_range(&self, pos: usize) -> Result<usize> {
        if pos == 0 || pos > self.get().text.len_chars() + 1 {
            bail!("Position {pos} out of range in {}", self.get().name);
        }
        Ok(pos - 1)
    }
}

impl<'new> WithLifetime<'new> for OpenBuffer<'_> {
    type Out = OpenBuffer<'new>;

    unsafe fn with_lifetime(self) -> Self::Out {
        std::mem::transmute(self)
    }
}

impl PartialEq<str> for OpenBuffer<'_> {
    fn eq(&self, other: &str) -> bool {
        self.get().text == other
    }
}

impl Deref for OpenBuffer<'_> {
    type Target = BufferData;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl DerefMut for OpenBuffer<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

/// The actual data of the buffer. Buffer local variables will be stored here
/// eventually.
#[derive(Debug)]
pub(crate) struct BufferData {
    pub(crate) name: String,
    pub(crate) text: TextBuffer,
    pub(crate) textprops: IntervalTree<'static>,
}

impl BufferData {
    pub fn textprops_with_lifetime<'new>(&mut self) -> &mut IntervalTree<'new> {
        unsafe { std::mem::transmute(&mut self.textprops) }
    }
}

#[derive(Debug)]
struct LispBufferInner {
    text_buffer: Mutex<Option<BufferData>>,
}

/// A lisp handle to a buffer. This is a just a reference type and does not give
/// access to the contents until it is locked and a `OpenBuffer` is returned.
#[derive(PartialEq, Eq, Trace)]
pub(crate) struct LispBuffer(GcHeap<LispBufferInner>);

derive_GcMoveable!(LispBuffer);

impl LispBuffer {
    pub(crate) fn create(name: String, block: &Block<true>) -> &LispBuffer {
        let buffer = unsafe { Self::new(name, block) };
        block.objects.alloc(buffer)
    }

    pub(crate) unsafe fn new(name: String, _: &Block<true>) -> LispBuffer {
        let textprops = IntervalTree::new();
        let new = LispBufferInner {
            text_buffer: Mutex::new(Some(BufferData { name, text: TextBuffer::new(), textprops })),
        };
        Self(GcHeap::new(new, true))
    }

    pub(crate) fn lock(&self) -> Result<OpenBuffer<'_>> {
        let guard = self.0.text_buffer.lock().unwrap();
        if guard.is_none() {
            bail!("selecting deleted buffer");
        }
        Ok(OpenBuffer { data: guard, back_ref: self })
    }
}

impl PartialEq for LispBufferInner {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl PartialEq<OpenBuffer<'_>> for LispBuffer {
    fn eq(&self, other: &OpenBuffer) -> bool {
        other.back_ref == self
    }
}

impl PartialEq<LispBuffer> for OpenBuffer<'_> {
    fn eq(&self, other: &LispBuffer) -> bool {
        self.back_ref == other
    }
}

impl Eq for LispBufferInner {}

impl Display for LispBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let data = self.0.text_buffer.lock().unwrap();
        let name = match data.as_ref() {
            Some(buf) => &buf.name,
            None => "deleted buffer",
        };
        write!(f, "#<{name}>")
    }
}

impl std::fmt::Debug for LispBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Trace for LispBufferInner {
    fn trace(&self, state: &mut GcState) {
        // Implement once we hold gc data in the buffer
        let buf = self.text_buffer.lock().unwrap();
        if let Some(buf) = buf.as_ref() {
            buf.textprops.trace(state);
        }
    }
}

impl<'new> LispBuffer {
    pub(in crate::core) fn clone_in<const C: bool>(
        &self,
        _: &'new Block<C>,
    ) -> Gc<&'new LispBuffer> {
        unsafe { self.with_lifetime().tag() }
    }
}
