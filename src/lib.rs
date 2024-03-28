#[cfg(all(not(target_env = "msvc"), not(miri)))]
#[global_allocator]
#[doc(hidden)]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[macro_use]
pub mod macros;
#[macro_use]
pub mod core;
#[macro_use]
mod debug;
mod alloc;
mod arith;
pub mod buffer;
mod bytecode;
mod casefiddle;
mod character;
mod data;
pub mod editfns;
pub mod emacs;
mod eval;
mod fileio;
mod floatfns;
pub mod fns;
mod interpreter;
pub mod intervals;
mod keymap;
mod lread;
mod print;
mod reader;
mod search;
pub mod textprops;
mod threads;
mod timefns;


pub use rune_core::macros::root;
