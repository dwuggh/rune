use anyhow::{bail, Result};
use face_types::{
    face::{Face, FaceMap},
    font::FontSystem,
    peniko::{self, Color},
};
use rune_core::macros::list;
use rune_macros::defun;

use crate::{
    alloc::list,
    core::{cons::Cons, object::{IntoObject, LispFrame, Object, ObjectType, Symbol, TagType, TRUE}},
    data::symbol_name,
    fns::eq,
    intern, Context, Env, Rt, NIL,
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

    Ok(())
}

#[defun]
fn internal_set_lisp_face_attribute_from_resource<'ob>(
    face: Object<'ob>,
    attr: Symbol<'ob>,
    value: Object<'ob>,
    frame: Option<Object<'ob>>,
    env: &mut Rt<Env>,
    cx: &'ob Context,
) -> Result<()> {
    internal_set_lisp_face_attribute(face, attr, value, frame, env, cx)
}

#[defun]
fn internal_face_x_get_resource(resource: Object, class: Object, frame: Option<Object>) {}

#[defun]
fn internal_get_lisp_face_attribute<'ob>(
    symbol: Object<'ob>,
    keyword: Symbol<'ob>,
    frame: Option<Object<'ob>>,
    env: &mut Rt<Env>,
    cx: &'ob Context,
) -> Result<Object<'ob>> {
    let face_name = face_name(symbol)?;
    let Some(face) = env.faces.get_mut(face_name) else { bail!("not a face") };
    let result = match symbol_name(keyword) {
        ":height" => cx.add(face.font_height as f64),
        ":width" => cx.add(face.swidth),
        ":family" => cx.add(face.families_to_string()),
        ":background" => {
            face.background_color.map(|c| cx.add(Face::color_to_string(&c))).unwrap_or(NIL)
        }
        _ => NIL,
    };
    Ok(result)
}

#[defun]
fn internal_lisp_face_attribute_values<'ob>(
    attr: Symbol<'ob>,
    env: &mut Rt<Env>,
    cx: &'ob Context,
) -> Result<Object<'ob>> {
    let result = match symbol_name(attr) {
        ":underline" | ":coverline" | ":strike-through" | "inverse-video" | "extend" => {
            list(&[TRUE, NIL], cx)
        }
        _ => NIL,
    };
    Ok(result)
}

#[defun]
fn internal_merge_in_global_face<'ob>(
    face: Symbol<'ob>,
    frame: Option<Object>,
    env: &mut Rt<Env>,
    cx: &'ob Context,
) {
    // we probably don't want this function
}

#[defun]
fn face_attribute_relative_p<'ob>(attr: Symbol<'ob>, value: Object<'ob>, cx: &'ob Context) -> bool {
    let unspec = cx.add(intern("unspecified", cx));
    if eq(value, unspec) {
        return true;
    } else if symbol_name(attr) == ":height" {
        return true;
    } else {
        return false;
    }
}

#[defun]
fn merge_face_attribute<'ob>(
    attr: Symbol<'ob>,
    value1: Object<'ob>,
    value2: Object<'ob>,
    cx: &'ob Context,
) -> Object<'ob> {
    return value2;
}

#[defun]
fn internal_lisp_face_equal_p(face1: Object, face2: Object, frame: Option<Object>) -> bool {
    eq(face1, face2)
}

#[defun]
fn internal_lisp_face_empty_p(face: Object, frame: Option<Object>, env: &Rt<Env>) -> bool {
    todo!()
}

#[defun]
fn color_distance(color1: Object, color2: Object, frame: Option<Object>, metric: Option<Object>) -> Result<i64> {
    let c1 = parse_color(color1)?.to_rgba8();
    let c2 = parse_color(color2)?.to_rgba8();
    let result = c1.r.abs_diff(c2.r) + c1.g.abs_diff(c2.g) + c1.b.abs_diff(c2.b);
    Ok(result as i64)
}

fn parse_color(color: Object) -> Result<Color> {
    match color.untag() {
        ObjectType::String(s) => {
            Ok(peniko::color::parse_color(s).map(|c| Color::new(c.components))?)
        }
        ObjectType::Cons(l) => {
            let r: i64 = l.car().try_into()?;
            let l: &Cons = l.cdr().try_into()?;
            let g: i64 = l.car().try_into()?;
            let l: &Cons = l.cdr().try_into()?;
            let b: i64 = l.car().try_into()?;
            let color = Color::from_rgb8(r as u8, g as u8, b as u8);
            Ok(color)
        }
        _ => Err(anyhow::anyhow!("wrong type")),
    }
}

#[defun]
fn face_attributes_as_vector<'ob>(plist: Object, env: &Rt<Env>, cx: &'ob Context) -> Result<Object<'ob>> {
    todo!()
}

#[defun]
fn face_font(
    face: Object,
    frame: Option<Object>,
    character: Option<Object>,
    env: &Rt<Env>,
) -> Result<Option<String>> {
    let face_name = face_name(face)?;
    let Some(face) = env.faces.get(face_name) else { bail!("not a face") };
    Ok(Some(face.families_to_string()))
}

#[defun]
fn display_supports_face_attributes_p(attributes: Object, display: Option<Object>) -> bool {
    true
}

#[defun]
fn internal_set_font_selection_order(order: Object) -> Result<()> {
    // TODO
    Ok(())
}

#[defun]
fn internal_set_alternative_font_family_alist(alist: Object) -> Result<()> {
    // TODO
    Ok(())
}

#[defun]
fn internal_set_alternative_font_registry_alist(alist: Object) -> Result<()> {
    // TODO
    Ok(())
}

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
