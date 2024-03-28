//! Utilities for variables and values.
use crate::core::{
    cons::Cons,
    env::{interned_symbols, sym, Env},
    error::{Type, TypeError},
    gc::{Context, Rt},
    object::{List, ListType, Number, Object, ObjectType, SubrFn, Symbol, WithLifetime, NIL},
};
use anyhow::{anyhow, Result};
use rune_core::hashmap::HashSet;
use rune_macros::defun;
use std::sync::Mutex;
use std::sync::OnceLock;

static FEATURES: OnceLock<Mutex<HashSet<Symbol<'static>>>> = OnceLock::new();

/// Rust translation of the `features` variable: A list of symbols are the features
/// of the executing Emacs. Used by [`featurep`](`crate::fns::featurep`) and [`require`](`crate::fns::require`),
/// altered by [`provide`]. Vended through a helper function to avoid calling `get_or_init` on each of the calls
/// to `lock()` on the Mutex.
///
/// TODO: Use `LazyLock`: <https://github.com/CeleritasCelery/rune/issues/34>
pub fn features() -> &'static Mutex<HashSet<Symbol<'static>>> {
    FEATURES.get_or_init(Mutex::default)
}

#[defun]
pub fn fset<'ob>(symbol: Symbol<'ob>, definition: Object) -> Result<Symbol<'ob>> {
    if definition.is_nil() {
        symbol.unbind_func();
    } else {
        let func = definition.try_into()?;
        let map = interned_symbols().lock().unwrap();
        map.set_func(symbol, func)?;
    }
    Ok(symbol)
}

#[defun]
pub fn defalias<'ob>(
    symbol: Symbol<'ob>,
    definition: Object,
    _docstring: Option<&str>,
) -> Result<Symbol<'ob>> {
    fset(symbol, definition)
}

#[defun]
pub fn set<'ob>(
    place: Symbol,
    newlet: Object<'ob>,
    env: &mut Rt<Env>,
) -> Result<Object<'ob>> {
    env.set_var(place, newlet)?;
    Ok(newlet)
}

#[defun]
pub fn put<'ob>(
    symbol: Symbol,
    propname: Symbol,
    value: Object<'ob>,
    env: &mut Rt<Env>,
) -> Object<'ob> {
    env.set_prop(symbol, propname, value);
    value
}

#[defun]
pub fn get<'ob>(
    symbol: Symbol,
    propname: Symbol,
    env: &Rt<Env>,
    cx: &'ob Context,
) -> Object<'ob> {
    match env.props.get(symbol) {
        Some(plist) => match plist.iter().find(|x| x.0 == propname) {
            Some(element) => cx.bind(element.1.bind(cx)),
            None => NIL,
        },
        None => NIL,
    }
}

#[defun]
pub fn local_variable_if_set_p(_sym: Symbol) -> bool {
    // TODO: Implement buffer locals
    false
}

#[defun]
pub fn default_value<'ob>(
    symbol: Symbol,
    env: &Rt<Env>,
    cx: &'ob Context,
) -> Result<Object<'ob>> {
    // TODO: Implement buffer locals
    symbol_value(symbol, env, cx).ok_or_else(|| anyhow!("Void variable: {symbol}"))
}

#[defun]
pub fn symbol_function<'ob>(symbol: Symbol, cx: &'ob Context) -> Object<'ob> {
    match symbol.func(cx) {
        Some(f) => f.into(),
        None => NIL,
    }
}

#[defun]
pub fn symbol_value<'ob>(
    symbol: Symbol,
    env: &Rt<Env>,
    cx: &'ob Context,
) -> Option<Object<'ob>> {
    env.vars.get(symbol).map(|x| x.bind(cx))
}

#[defun]
pub fn symbol_name(symbol: Symbol<'_>) -> &str {
    symbol.get().name()
}

#[defun]
pub fn null(obj: Object) -> bool {
    obj.is_nil()
}

#[defun]
pub fn fboundp(symbol: Symbol) -> bool {
    symbol.has_func()
}

#[defun]
pub fn fmakunbound(symbol: Symbol) -> Symbol {
    symbol.unbind_func();
    symbol
}

#[defun]
pub fn boundp(symbol: Symbol, env: &Rt<Env>) -> bool {
    env.vars.get(symbol).is_some()
}

#[defun]
pub fn makunbound<'ob>(symbol: Symbol<'ob>, env: &mut Rt<Env>) -> Symbol<'ob> {
    env.vars.remove(symbol);
    symbol
}

#[defun]
pub fn default_boundp(symbol: Symbol, env: &Rt<Env>) -> bool {
    env.vars.get(symbol).is_some()
}

#[defun]
pub fn listp(object: Object) -> bool {
    matches!(object.untag(), ObjectType::NIL | ObjectType::Cons(_))
}

#[defun]
pub fn nlistp(object: Object) -> bool {
    !listp(object)
}

#[defun]
pub fn symbolp(object: Object) -> bool {
    matches!(object.untag(), ObjectType::Symbol(_))
}

#[defun]
pub fn functionp(object: Object) -> bool {
    match object.untag() {
        ObjectType::ByteFn(_) | ObjectType::SubrFn(_) => true,
        ObjectType::Cons(cons) => cons.car() == sym::CLOSURE,
        ObjectType::Symbol(sym) => sym.has_func(),
        _ => false,
    }
}

#[defun]
pub fn subrp(object: Object) -> bool {
    matches!(object.untag(), ObjectType::SubrFn(_))
}

#[defun]
pub fn stringp(object: Object) -> bool {
    matches!(object.untag(), ObjectType::String(_))
}

#[defun]
pub fn numberp(object: Object) -> bool {
    matches!(object.untag(), ObjectType::Int(_) | ObjectType::Float(_))
}

#[defun]
pub fn markerp(_: Object) -> bool {
    // TODO: implement
    false
}

#[defun]
pub fn vectorp(object: Object) -> bool {
    matches!(object.untag(), ObjectType::Vec(_))
}

#[defun]
pub fn recordp(object: Object) -> bool {
    matches!(object.untag(), ObjectType::Record(_))
}

#[defun]
pub fn consp(object: Object) -> bool {
    matches!(object.untag(), ObjectType::Cons(_))
}

#[defun]
pub fn keywordp(object: Object) -> bool {
    match object.untag() {
        ObjectType::Symbol(s) => s.name().starts_with(':'),
        _ => false,
    }
}

#[defun]
pub fn integerp(object: Object) -> bool {
    matches!(object.untag(), ObjectType::Int(_))
}

#[defun]
pub fn floatp(object: Object) -> bool {
    matches!(object.untag(), ObjectType::Float(_))
}

#[defun]
pub fn atom(object: Object) -> bool {
    !consp(object)
}

#[defun]
fn byte_code_function_p(object: Object) -> bool {
    matches!(object.untag(), ObjectType::ByteFn(_))
}

#[defun]
fn subr_native_elisp_p(_: Object) -> bool {
    false
}

#[defun]
fn bufferp(_object: Object) -> bool {
    // TODO: Implement once buffers are added
    false
}

#[defun]
pub fn multibyte_string_p(object: Object) -> bool {
    matches!(object.untag(), ObjectType::String(_))
}

#[defun]
fn string_to_number<'ob>(string: &str, base: Option<i64>, cx: &'ob Context) -> Number<'ob> {
    // TODO: Handle trailing characters, which should be ignored
    let base = base.unwrap_or(10);
    let string = string.trim();
    match i64::from_str_radix(string, base as u32) {
        Ok(x) => x.into(),
        Err(_) => match string.parse::<f64>() {
            Ok(x) => cx.add_as(x),
            Err(_) => 0.into(),
        },
    }
}

#[defun]
pub fn defvar<'ob>(
    symbol: Symbol,
    initvalue: Option<Object<'ob>>,
    _docstring: Option<&str>,
    env: &mut Rt<Env>,
) -> Result<Object<'ob>> {
    let value = initvalue.unwrap_or_default();
    set(symbol, value, env)
}

#[defun]
pub fn make_variable_buffer_local(variable: Symbol) -> Symbol {
    // TODO: Implement
    variable
}

#[defun]
fn subr_arity<'ob>(subr: &SubrFn, cx: &'ob Context) -> Object<'ob> {
    let min = subr.args.required as usize;
    let max: Object = {
        if subr.args.rest {
            sym::MANY.into()
        } else {
            (min + subr.args.optional as usize).into()
        }
    };
    Cons::new(min, max, cx).into()
}

#[defun]
fn ash(value: i64, count: i64) -> i64 {
    let shift = if count >= 0 { std::ops::Shl::shl } else { std::ops::Shr::shr };
    let result = shift(value.abs(), count.abs());
    if value >= 0 {
        result
    } else {
        -result
    }
}

#[defun]
pub fn aset<'ob>(
    array: Object<'ob>,
    idx: usize,
    newlet: Object<'ob>,
) -> Result<Object<'ob>> {
    match array.untag() {
        ObjectType::Vec(vec) => {
            let vec = vec.try_mut()?;
            if idx < vec.len() {
                vec[idx].set(newlet);
                Ok(newlet)
            } else {
                let len = vec.len();
                Err(anyhow!("index {idx} is out of bounds. Length was {len}"))
            }
        }
        ObjectType::Record(vec) => {
            let vec = vec.try_mut()?;
            if idx < vec.len() {
                vec[idx].set(newlet);
                Ok(newlet)
            } else {
                let len = vec.len();
                Err(anyhow!("index {idx} is out of bounds. Length was {len}"))
            }
        }
        x => Err(TypeError::new(Type::Sequence, x).into()),
    }
}

#[defun]
pub fn aref<'ob>(array: Object<'ob>, idx: usize, cx: &'ob Context) -> Result<Object<'ob>> {
    match array.untag() {
        ObjectType::Vec(vec) => match vec.get(idx) {
            Some(x) => Ok(x.get()),
            None => {
                let len = vec.len();
                Err(anyhow!("index {idx} is out of bounds. Length was {len}"))
            }
        },
        ObjectType::Record(vec) => match vec.get(idx) {
            Some(x) => Ok(x.get()),
            None => {
                let len = vec.len();
                Err(anyhow!("index {idx} is out of bounds. Length was {len}"))
            }
        },
        ObjectType::String(string) => match string.chars().nth(idx) {
            Some(x) => Ok((i64::from(x as u32)).into()),
            None => {
                let len = string.len();
                Err(anyhow!("index {idx} is out of bounds. Length was {len}"))
            }
        },
        ObjectType::ByteString(string) => match string.get(idx) {
            Some(x) => Ok((i64::from(*x)).into()),
            None => {
                let len = string.len();
                Err(anyhow!("index {idx} is out of bounds. Length was {len}"))
            }
        },
        ObjectType::ByteFn(fun) => match fun.index(idx, cx) {
            Some(x) => Ok(x),
            None => Err(anyhow!("index {idx} is out of bounds")),
        },
        x => Err(TypeError::new(Type::Sequence, x).into()),
    }
}

#[defun]
fn type_of(object: Object) -> Object {
    match object.untag() {
        ObjectType::Int(_) => sym::INTEGER.into(),
        ObjectType::Float(_) => sym::FLOAT.into(),
        ObjectType::Symbol(_) => sym::SYMBOL.into(),
        ObjectType::Cons(_) => sym::CONS.into(),
        ObjectType::Vec(_) => sym::VECTOR.into(),
        ObjectType::Record(x) => x.first().expect("record was missing type").get(),
        ObjectType::ByteFn(_) => sym::COMPILED_FUNCTION.into(),
        ObjectType::HashTable(_) => sym::HASH_TABLE.into(),
        ObjectType::String(_) | ObjectType::ByteString(_) => sym::STRING.into(),
        ObjectType::SubrFn(_) => sym::SUBR.into(),
        ObjectType::Buffer(_) => sym::BUFFER.into(),
    }
}

#[defun]
pub fn indirect_function<'ob>(object: Object<'ob>, cx: &'ob Context) -> Object<'ob> {
    match object.untag() {
        ObjectType::Symbol(sym) => match sym.follow_indirect(cx) {
            Some(func) => func.into(),
            None => NIL,
        },
        _ => object,
    }
}

#[defun]
pub fn provide<'ob>(feature: Symbol<'ob>, _subfeatures: Option<&Cons>) -> Symbol<'ob> {
    let mut features = features().lock().unwrap();
    // TODO: SYMBOL - need to trace this
    let feat = unsafe { feature.with_lifetime() };
    features.insert(feat);
    feature
}

#[defun]
pub fn car(list: List) -> Object {
    match list.untag() {
        ListType::Cons(cons) => cons.car(),
        ListType::Nil => NIL,
    }
}

#[defun]
pub fn cdr(list: List) -> Object {
    match list.untag() {
        ListType::Cons(cons) => cons.cdr(),
        ListType::Nil => NIL,
    }
}

#[defun]
pub fn car_safe(object: Object) -> Object {
    match object.untag() {
        ObjectType::Cons(cons) => cons.car(),
        _ => NIL,
    }
}

#[defun]
pub fn cdr_safe(object: Object) -> Object {
    match object.untag() {
        ObjectType::Cons(cons) => cons.cdr(),
        _ => NIL,
    }
}

#[defun]
pub fn setcar<'ob>(cell: &Cons, newcar: Object<'ob>) -> Result<Object<'ob>> {
    cell.set_car(newcar)?;
    Ok(newcar)
}

#[defun]
pub fn setcdr<'ob>(cell: &Cons, newcdr: Object<'ob>) -> Result<Object<'ob>> {
    cell.set_cdr(newcdr)?;
    Ok(newcdr)
}

#[defun]
pub fn cons<'ob>(car: Object, cdr: Object, cx: &'ob Context) -> Object<'ob> {
    Cons::new(car, cdr, cx).into()
}

// Symbol with position
#[defun]
fn bare_symbol(sym: Symbol) -> Symbol {
    // TODO: implement
    sym
}

#[defun]
fn symbol_with_pos_p(_sym: Object) -> bool {
    // TODO: implement
    false
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ash() {
        assert_eq!(ash(4, 1), 8);
        assert_eq!(ash(4, -1), 2);
        assert_eq!(ash(-8, -1), -4);
        assert_eq!(ash(256, -8), 1);
        assert_eq!(ash(-8, 1), -16);
    }
}

defsym!(MANY);
defsym!(INTEGER);
defsym!(SYMBOL);
defsym!(COMPILED_FUNCTION);
defsym!(HASH_TABLE);
defsym!(BUFFER);
defsym!(SUBR);
