use std::collections::HashMap;
use std::sync::Mutex;

use rune_macros::Trace;

use crate::core::gc::IntoRoot;
use crate::derive_GcMoveable;
use crate::{
    core::gc::{Block, GcHeap, Slot, Trace},
    frame::FrameConfig,
};

use super::{Gc, LispWindow, NIL, Object, TagType, WindowData, WithLifetime};

#[derive(PartialEq, Eq, Debug, Trace)]
pub struct LispFrame(GcHeap<LispFrameInner<'static>>);
derive_GcMoveable!(LispFrame);

#[derive(Debug)]
pub struct LispFrameInner<'ob> {
    data: Mutex<FrameData<'ob>>,
}

#[derive(Debug)]
pub enum ComponentData<'ob> {
    Window(WindowData<'ob>),
    Modeline,
    Bar,
}

#[derive(Debug)]
pub(crate) struct Component<'ob> {
    pub(crate) id: u64,
    pub(crate) data: ComponentData<'ob>,
}

impl Trace for Component<'_> {
    fn trace(&self, state: &mut crate::core::gc::GcState) {
        match &self.data {
            ComponentData::Window(window_data) => window_data.trace(state),
            _ => (),
        }
    }
}

impl<'new> IntoRoot<Component<'new>> for Component<'_> {
    unsafe fn into_root(self) -> Component<'new> {
        let result: Component<'new> = std::mem::transmute(self);
        result
    }
}

#[derive(Debug)]
pub(crate) struct FrameData<'ob> {
    pub(crate) config: FrameConfig,
    pub(crate) params: Slot<Object<'ob>>,
    pub(crate) components: HashMap<u64, Component<'ob>>,
}

impl Trace for FrameData<'_> {
    fn trace(&self, state: &mut crate::core::gc::GcState) {
        self.params.trace(state);
        for (_id, data) in self.components.iter() {
            data.trace(state);
        }
    }
}

impl Trace for LispFrameInner<'_> {
    fn trace(&self, state: &mut crate::core::gc::GcState) {
        self.data.lock().unwrap().trace(state);
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

impl Eq for LispFrameInner<'_> {}

impl<'ob> FrameData<'ob> {
    fn new(config: FrameConfig, params: Slot<Object<'ob>>) -> Self {
        let windows = HashMap::new();
        // let id = config.layout.main;
        Self { config, params, components: windows }
    }
}

impl<'new> LispFrame {
    pub(in crate::core) fn clone_in<const C: bool>(
        &self,
        _: &'new Block<C>,
    ) -> Gc<&'new LispFrame> {
        unsafe { self.with_lifetime().tag() }
    }
}

impl<'ob> LispFrameInner<'ob> {
    pub fn new(config: FrameConfig, params: Slot<Object<'ob>>) -> Self {
        let data = FrameData::new(config, params);
        let data = Mutex::new(data);
        LispFrameInner { data }
    }
}

impl LispFrame {
    pub fn create<'ob>(
        frame: FrameConfig,
        params: Slot<Object<'ob>>,
        block: &'ob Block<true>,
    ) -> &'ob Self {
        let frame = unsafe { Self::new(frame, params, true) };
        block.objects.alloc(frame)
    }

    pub unsafe fn new<'ob>(frame: FrameConfig, params: Slot<Object<'ob>>, constant: bool) -> Self {
        let params = params.with_lifetime();
        let new = GcHeap::new(LispFrameInner::new(frame, params), constant);

        Self(new)
    }

    pub fn data(&self) -> std::sync::MutexGuard<'_, FrameData<'static>> {
        let guard = self.0.data.lock().unwrap();
        guard
    }
}
