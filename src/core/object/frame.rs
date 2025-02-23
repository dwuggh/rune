use std::{cell::RefCell, sync::Mutex};

use rune_macros::Trace;

use crate::derive_GcMoveable;
use crate::{
    core::gc::{Block, GcHeap, Slot, Trace},
    frame::Frame,
    window::Window,
};

use super::{Gc, Object, TagType, WithLifetime};

#[derive(PartialEq, Eq, Debug, Trace)]
pub struct LispFrame(GcHeap<LispFrameInner<'static>>);
derive_GcMoveable!(LispFrame);

#[derive(Debug, Trace)]
pub struct LispFrameInner<'ob> {
    #[no_trace]
    frame: Mutex<Frame>,
    params: Slot<Object<'ob>>,
}

impl<'ob> PartialEq for LispFrameInner<'ob> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

// FIXME is Slot<Object> thread-safe?
unsafe impl Sync for LispFrameInner<'_> {}
unsafe impl Send for LispFrameInner<'_> {}

impl Eq for LispFrameInner<'_> {}

impl<'new> LispFrame {
    pub(in crate::core) fn clone_in<const C: bool>(
        &self,
        _: &'new Block<C>,
    ) -> Gc<&'new LispFrame> {
        unsafe { self.with_lifetime().tag() }
    }
}

impl<'ob> LispFrameInner<'ob> {
    pub fn new(f: Frame, params: Slot<Object<'ob>>) -> Self {
        let frame = Mutex::new(f);
        LispFrameInner { frame, params }
    }
}

impl LispFrame {
    pub fn create<'ob>(
        frame: Frame,
        params: Slot<Object<'ob>>,
        block: &'ob Block<true>,
    ) -> &'ob Self {
        let frame = unsafe { Self::new(frame, params, true) };
        block.objects.alloc(frame)
    }

    pub unsafe fn new<'ob>(frame: Frame, params: Slot<Object<'ob>>, constant: bool) -> Self {
        let params = params.with_lifetime();
        let new = GcHeap::new(LispFrameInner::new(frame, params), constant);

        Self(new)
    }

    pub fn data(&self) -> std::sync::MutexGuard<'_, Frame> {
        self.0.frame.lock().unwrap()
    }

    pub fn params(&self) -> Object {
        self.0.params.as_obj()
    }
}
