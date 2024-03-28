use crate::intervals::IntervalTree;

use super::gc::{Context, ObjectMap, Rto, Slot};
use super::object::{LispBuffer, Object, OpenBuffer, Symbol, WithLifetime};
use anyhow::{anyhow, Result};
use rune_core::hashmap::IndexMap;
use rune_macros::Trace;

mod stack;
mod symbol_map;
pub use stack::*;
pub use symbol_map::*;

type PropertyMap<'a> = ObjectMap<Slot<Symbol<'a>>, Vec<(Slot<Symbol<'a>>, Slot<Object<'a>>)>>;
#[derive(Debug, Default, Trace)]
pub struct Env<'a> {
    pub vars: ObjectMap<Slot<Symbol<'a>>, Slot<Object<'a>>>,
    pub props: PropertyMap<'a>,
    pub catch_stack: Vec<Slot<Object<'a>>>,
    exception: (Slot<Object<'a>>, Slot<Object<'a>>),
    #[no_trace]
    exception_id: u32,
    binding_stack: Vec<(Slot<Symbol<'a>>, Option<Slot<Object<'a>>>)>,
    pub match_data: Slot<Object<'a>>,
    #[no_trace]
    pub current_buffer: Option<OpenBuffer<'a>>,
    #[no_trace]
    pub buffer_textprops: IndexMap<Object<'a>, IntervalTree<'a>>,
    pub stack: LispStack<'a>,
}

// RootedEnv created by #[derive(Trace)]
impl<'a> RootedEnv<'a> {
    pub fn set_var(&mut self, sym: Symbol, value: Object) -> Result<()> {
        if sym.is_const() {
            Err(anyhow!("Attempt to set a constant symbol: {sym}"))
        } else {
            self.vars.insert(sym, value);
            Ok(())
        }
    }

    pub fn set_prop(&mut self, symbol: Symbol, propname: Symbol, value: Object) {
        match self.props.get_mut(symbol) {
            Some(plist) => match plist.iter_mut().find(|x| x.0 == propname) {
                Some(x) => x.1.set(value),
                None => plist.push((propname, value)),
            },
            None => {
                self.props.insert(symbol, vec![(propname, value)]);
            }
        }
    }

    pub fn set_exception(&mut self, tag: Object, data: Object) -> u32 {
        self.exception.0.set(tag);
        self.exception.1.set(data);
        self.exception_id += 1;
        self.exception_id
    }

    pub fn get_exception(&self, id: u32) -> Option<(&Rto<Object<'a>>, &Rto<Object<'a>>)> {
        (id == self.exception_id).then_some((&self.exception.0, &self.exception.1))
    }

    pub fn varbind(&mut self, var: Symbol, value: Object, cx: &Context) {
        let prev_value = self.vars.get(var).map(|x| x.bind(cx));
        self.binding_stack.push((var, prev_value));
        self.vars.insert(var, value);
    }

    pub fn unbind(&mut self, count: u16, cx: &Context) {
        for _ in 0..count {
            match self.binding_stack.bind_mut(cx).pop() {
                Some((sym, val)) => match val {
                    Some(val) => self.vars.insert(*sym, *val),
                    None => self.vars.remove(*sym),
                },
                None => panic!("Binding stack was empty"),
            }
        }
    }

    pub fn defvar(&mut self, var: Symbol, value: Object) -> Result<()> {
        // TOOD: Handle `eval-sexp` on defvar, which should always update the
        // value
        if self.vars.get(var).is_none() {
            self.set_var(var, value)?;
            var.make_special();
        }

        // If this variable was unbound previously in the binding stack,
        // we will bind it to the new value
        for binding in &mut *self.binding_stack {
            if binding.0 == var && binding.1.is_none() {
                binding.1.set(Some(value));
            }
        }
        Ok(())
    }

    #[inline]
    pub fn buffer_textprops(&self, buffer: Object<'a>) -> Option<&IntervalTree<'a>> {
        self.buffer_textprops.get(&buffer)
    }

    #[inline]
    pub fn buffer_textprops_mut(&mut self, buffer: Object<'a>) -> &mut IntervalTree<'a> {
        self.buffer_textprops.entry(buffer).or_insert(IntervalTree::new())
    }

    pub fn set_buffer(&mut self, buffer: &LispBuffer) -> Result<()> {
        if let Some(current) = &self.current_buffer {
            if buffer == current {
                return Ok(());
            }
        }
        // SAFETY: We are not dropping the buffer until we have can trace it
        // with the garbage collector
        let lock = unsafe { buffer.lock()?.with_lifetime() };
        self.current_buffer = Some(lock);
        Ok(())
    }

    pub fn with_buffer<T>(
        &self,
        buffer: Option<&LispBuffer>,
        func: impl Fn(&OpenBuffer) -> T,
    ) -> Option<T> {
        match (&self.current_buffer, buffer) {
            (Some(_), None) => Some(func(self.current_buffer.as_ref().unwrap())),
            (Some(current), Some(buffer)) if current == buffer => {
                Some(func(self.current_buffer.as_ref().unwrap()))
            }
            (_, Some(buffer)) => {
                if let Ok(buffer) = buffer.lock().as_mut() {
                    Some(func(buffer))
                } else {
                    None
                }
            }
            (None, None) => None,
        }
    }

    pub fn with_buffer_mut<T>(
        &mut self,
        buffer: Option<&LispBuffer>,
        func: impl Fn(&mut OpenBuffer) -> T,
    ) -> Option<T> {
        match (&self.current_buffer, buffer) {
            (Some(current), Some(buffer)) if current == buffer => {
                Some(func(self.current_buffer.as_mut().unwrap()))
            }
            (Some(_), None) => Some(func(self.current_buffer.as_mut().unwrap())),
            (_, Some(buffer)) => {
                if let Ok(buffer) = buffer.lock().as_mut() {
                    Some(func(buffer))
                } else {
                    None
                }
            }
            (None, None) => None,
        }
    }
}
