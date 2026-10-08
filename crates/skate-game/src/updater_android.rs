//! Updates are handled by the app store; the desktop helper does not exist here.
use bevy::prelude::*;

#[derive(Resource, Default)]
pub(crate) struct Updater;

impl Updater {
    pub(crate) fn open(&mut self, _automatic: bool) -> String {
        "Updates are not available on Android.".into()
    }
}

pub(crate) struct UpdaterPlugin;
impl Plugin for UpdaterPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Updater>();
    }
}

pub(crate) fn recover() -> Result<bool, String> {
    Ok(false)
}
