use anyhow::Result;
use rune_macros::defun;

use crate::{
    Context, Gc,
    buffer::get_buffer,
    core::object::{IntoObject, LispWindow, Object, ObjectType},
};

#[defun]
fn windowp(object: Object) -> bool {
    matches!(object.untag(), ObjectType::Window(_))
}

#[defun]
fn window_frame(window: Object) -> Result<Object> {
    match window.untag() {
        ObjectType::Window(w) => {
            let frame: Object = w.get_frame().into();
            Ok(frame)
        }
        _ => todo!(),
    }
}

#[defun]
fn seleted_window<'ob>() -> Result<Object<'ob>> {
    todo!()
}

#[defun]
fn set_window_buffer<'ob>(
    window: Gc<&LispWindow>,
    buffer_or_name: Object<'ob>,
    keep_margins: Option<Object<'ob>>,
    cx: &'ob Context,
) -> Result<()> {
    let buf = get_buffer(buffer_or_name, cx)?;
    let ObjectType::Buffer(buf) = buf.untag() else { return Ok(()) };
    let w = window.untag();

    todo!()
}
