use anyhow::Result;
use rune_macros::Trace;

use crate::{
    core::{error::TypeError, gc::{Block, GcHeap, IntoRoot, Slot}},
    derive_GcMoveable,
};

use super::{Gc, LispFrame, Object, TagType, WithLifetime, NIL};

#[derive(PartialEq, Eq, Debug, Trace)]
pub struct LispWindow(GcHeap<LispWindowInner<'static>>);
derive_GcMoveable!(LispWindow);

#[derive(Debug, Trace)]
pub struct LispWindowInner<'ob> {
    id: u64,
    #[no_trace]
    config: WindowConfig,
    buffer: Slot<Object<'ob>>,
    params: Slot<Object<'ob>>,
    parent_frame: Slot<Object<'ob>>,
}

impl<'ob> PartialEq for LispWindowInner<'ob> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for LispWindowInner<'_> {}
impl<'new> LispWindow {
    pub(in crate::core) fn clone_in<const C: bool>(
        &self,
        _: &'new Block<C>,
    ) -> Gc<&'new LispWindow> {
        unsafe { self.with_lifetime().tag() }
    }
}

impl LispWindow {
    pub(crate) fn new(
        id: u64,
        buffer: Slot<Object>,
        frame: Slot<Object>,
        params: Object,
    ) -> Self {
        let config = WindowConfig::new();
        let params = unsafe { params.into_root() };
        let buffer = unsafe { buffer.with_lifetime() };
        let frame = unsafe { frame.with_lifetime() };
        let new = LispWindowInner { id, config, buffer, params, parent_frame: frame };
        Self(GcHeap::new(new, true))
    }

    pub(crate) fn get_frame(&self) -> Result<&LispFrame> {
        let frame = *self.0.parent_frame;
        match frame.untag() {
            super::ObjectType::Frame(f) => Ok(f),
            _ => Err(TypeError::new(crate::core::error::Type::Frame, frame).into())
        }
    }
}

#[derive(Debug)]
pub struct WindowConfig {
    /// A marker pointing to where in the text to start displaying.
    disp_start: u64,

    /// A marker pointing to where in the text point is in this window,
    /// used only when the window is not selected.
    /// This exists so that when multiple windows show one buffer
    /// each one can have its own value of point.
    point: u64,

    left: f32,
    top: f32,

    /// Line number and position of a line somewhere above the top of the
    /// screen.  If this field is zero, it means we don't have a base line.
    ///
    /// used in line-number-mode, ignore for now
    base_line_number: u64,
    base_line_pos: u64,
}

impl WindowConfig {
    fn new() -> Self {
        Self {
            disp_start: 1,
            point: 1,
            left: 0.,
            top: 0.,
            base_line_number: 0,
            base_line_pos: 0,
        }
    }
}
