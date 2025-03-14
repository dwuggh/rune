use anyhow::{bail, Result};
use face_types::{
    face::{Face, FaceMap}, font::FontSystem, fontdb, peniko
};
use rune_core::macros::list;
use rune_macros::defun;

use crate::{
    alloc::list,
    core::object::{LispFrame, Object, ObjectType, Symbol},
    data::symbol_name,
    Context, Env, Rt,
};

#[derive(Debug, Default)]
pub(crate) struct FaceManager {
    pub faces: FaceMap,
    font_manager: FontSystem,
}

impl FaceManager {
    pub(crate) fn init(&mut self) {
        self.font_manager.load_system_fonts();
    }

    fn insert(&mut self, face: Face) {
        self.faces.insert(face);
    }

    fn get(&self, name: &str) -> Option<&Face> {
        self.faces.get(name)
    }

    fn get_mut(&mut self, name: &str) -> Option<&mut Face> {
        self.faces.get_mut(name)
    }
}

#[defun]
fn dump_colors() {}

#[defun]
fn clear_face_cache(obj: Option<Object>) {}

#[defun]
fn color_values_from_color_spec<'ob>(color: &str, cx: &'ob Context) -> Result<Object<'ob>> {
    let color = peniko::color::parse_color(color)?;
    let rgba = &color.components;
    let re = list!(rgba[0] as f64, rgba[1] as f64, rgba[2] as f64; cx);
    Ok(re)
}

#[defun]
fn x_family_fonts<'ob>(
    family: Option<&str>,
    frame: Option<Object<'ob>>,
    env: &mut Rt<Env>,
    cx: &'ob Context,
) -> Result<Object<'ob>> {
    // TODO this is probably nowhere close to correct
    let font_manager = &env.faces.font_manager;
    let result: Vec<String> = font_manager
        .db
        .faces()
        .map(|x| x.families.iter().map(|(name, l)| name.clone()))
        .flatten()
        .collect();
    // println!("{:?}", result);
    let result: Vec<Object> = result.into_iter().map(|s| cx.add(s)).collect();
    let result = list(&result, cx);
    Ok(result)
}

#[defun]
fn internal_make_lisp_face<'ob>(
    face: Symbol<'ob>,
    frame: Option<Object<'ob>>,
    env: &mut Rt<Env>,
    cx: &'ob Context,
) {
    let face = Face { name: symbol_name(face).to_string(), ..Default::default() };
    env.faces.insert(face);
}

#[defun]
fn internal_lisp_face_p<'ob>(
    face: Object<'ob>,
    frame: Option<Object<'ob>>,
    env: &mut Rt<Env>,
    cx: &'ob Context,
) -> Result<bool> {
    match face.untag() {
        ObjectType::String(s) => Ok(env.faces.get(s).is_some()),
        ObjectType::Symbol(s) => Ok(env.faces.get(symbol_name(s)).is_some()),
        _ => bail!("wrong type argument"),
    }
}

#[defun]
fn internal_copy_lisp_face<'ob>(
    from: Object<'ob>,
    to: Object<'ob>,
    frame: Object<'ob>,
    new_frame: Object<'ob>,
    env: &mut Rt<Env>,
    cx: &'ob Context,
) {
    todo!()
}

#[defun]
fn internal_set_lisp_face_attribute<'ob>(
    face: Object<'ob>,
    attr: Symbol<'ob>,
    value: Object<'ob>,
    frame: Option<Object<'ob>>,
    env: &mut Rt<Env>,
    cx: &'ob Context,
) -> Result<()> {
    let face_name = face_name(face)?;
    let Some(face) = env.faces.get_mut(face_name) else { bail!("not a face") };
    match symbol_name(attr) {
        ":family" => {
            let family: &str = value.try_into()?;
            todo!()
            // face.set_font_families(&[family.try_into()?]);
        }
        ":height" => {
            let height: i64 = value.try_into()?;
            face.font_height = height as f32;
        }
        ":weight" => {
            todo!()
            // let weight: i64 = value.try_into()?;
            // face.weight = Some(weight)
        }
        _ => {
            todo!()
        }
    }

    todo!()
}

#[defun]
fn internal_face_x_get_resource(resource: Object, class: Object, frame: Option<Object>) {}

fn face_name(face: Object) -> Result<&str> {
    match face.untag() {
        ObjectType::String(s) => Ok(s),
        ObjectType::Symbol(s) => Ok(symbol_name(s)),
        _ => Err(anyhow::anyhow!("wrong type argument")),
    }
}

#[cfg(test)]
mod tests {
    use rune_core::macros::root;

    use super::*;
    use crate::{
        core::object::{Object, ObjectType},
        Context, RootSet,
    };

    #[test]
    fn test_x_family_fonts() {
        let roots = &RootSet::default();
        let mut context = Context::new(roots);
        let cx = &mut context;
        root!(env, new(Env), cx);

        // Initialize font manager
        env.faces.init();

        // Test with no family specified
        let result = x_family_fonts(None, None, env, cx).unwrap();
        assert!(matches!(result.untag(), ObjectType::Cons(_)));
    }
}
