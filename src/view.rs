use ac2_scene::view::{SplMode, SweepMode};
use std::ops::{Deref, DerefMut};

/// The phone's selected pane modes live outside the shared scene/axis state,
/// just as the desktop now keeps each pane's view in its layout tree.
#[derive(Clone, Copy)]
pub struct ViewState {
    pub shared: ac2_scene::ViewState,
    pub spl_mode: SplMode,
    pub sweep_mode: SweepMode,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            shared: ac2_scene::ViewState::default(),
            spl_mode: SplMode::Meter,
            sweep_mode: SweepMode::Response,
        }
    }
}

impl Deref for ViewState {
    type Target = ac2_scene::ViewState;

    fn deref(&self) -> &Self::Target {
        &self.shared
    }
}

impl DerefMut for ViewState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.shared
    }
}
