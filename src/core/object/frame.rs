use std::collections::HashMap;
use std::{cell::RefCell, sync::Mutex};

use rune_macros::Trace;

use crate::core::gc::IntoRoot;
use crate::derive_GcMoveable;
use crate::{
    core::gc::{Block, GcHeap, Slot, Trace},
    frame::FrameConfig,
};

use super::{Gc, LispWindow, Object, TagType, WithLifetime, NIL};

#[derive(PartialEq, Eq, Debug, Trace)]
pub struct LispFrame(GcHeap<LispFrameInner<'static>>);
derive_GcMoveable!(LispFrame);

#[derive(Debug)]
pub struct LispFrameInner<'ob> {
    frame: Mutex<FrameConfig>,
    params: Slot<Object<'ob>>,
    // parent: Option<Object<'ob>>,
    windows: HashMap<u64, Slot<Object<'ob>>>
}

impl Trace for LispFrameInner<'_> {
    fn trace(&self, state: &mut crate::core::gc::GcState) {
        self.params.trace(state);
        for (_, w) in self.windows.iter() {
            w.trace(state);
        }
    }
}

impl<'new> IntoRoot<LispFrameInner<'new>> for LispFrameInner<'_> {
    unsafe fn into_root(self) -> LispFrameInner<'new> {
        self.with_lifetime()
    }
}

impl<'new> WithLifetime<'new> for LispFrameInner<'_> {
    type Out = LispFrameInner<'new>;
    unsafe fn with_lifetime(self) -> LispFrameInner<'new> {
        let result: LispFrameInner<'new> = std::mem::transmute(self);
        result
    }
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
    pub fn new(f: FrameConfig, params: Slot<Object<'ob>>) -> Self {
        let frame = Mutex::new(f);
        let windows = HashMap::new();
        LispFrameInner { frame, params, windows }
    }
}

impl LispFrame {
    pub fn create<'ob>(
        frame: FrameConfig,
        params: Slot<Object<'ob>>,
        block: &'ob Block<true>,
    ) -> &'ob Self {
        let frame = unsafe { Self::new(frame, params, true) };
        let window = LispWindow::new(id, buffer, frame, NIL);
        block.objects.alloc(frame)
    }

    pub unsafe fn new<'ob>(frame: FrameConfig, params: Slot<Object<'ob>>, constant: bool) -> Self {
        let params = params.with_lifetime();
        let new = GcHeap::new(LispFrameInner::new(frame, params), constant);

        Self(new)
    }

    pub fn data(&self) -> std::sync::MutexGuard<'_, FrameConfig> {
        self.0.frame.lock().unwrap()
    }

    pub fn params(&self) -> Object {
        self.0.params.as_obj()
    }
}
