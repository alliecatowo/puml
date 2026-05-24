mod activity;
mod chart;
mod class;
mod component;
mod generic;
mod helpers;
mod sequence;
mod state;
mod timing;

pub use activity::*;
pub use chart::*;
pub use class::*;
pub use component::*;
pub use generic::*;
pub use sequence::*;
pub use state::*;
pub use timing::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkinParamSupport<V> {
    SupportedNoop,
    SupportedWithValue(V),
    UnsupportedKey,
    UnsupportedValue,
}
