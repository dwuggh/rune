//! The core modules that define the primitive types and functionality of the
//! language.
#[macro_use]
pub mod cons;
pub mod env;
#[macro_use]
pub mod error;
pub mod object;
#[macro_use]
pub mod gc;

pub use object::Object;
pub use gc::Context;
