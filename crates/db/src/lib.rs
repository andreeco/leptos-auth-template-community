#![forbid(unsafe_code)]

pub mod entities;

pub use entities::*;

pub mod prelude {
    pub use crate::entities::prelude::*;
}
