use rune_macros::defun;
use anyhow::Result;

use crate::core::object::{Object, ObjectType};


#[defun]
fn framep(object: Object) -> bool {
    matches!(object.untag(), ObjectType::Window(_))
}

#[defun]
fn window_frame(window: Object) -> Result<Object> {
    match window.untag() {
        ObjectType::Window(w) => w.get_frame().map(|f| f.into()),
        _ => todo!()
    }
}

#[defun]
fn seleted_window<'ob>() -> Result<Object<'ob>> {
    todo!()
}