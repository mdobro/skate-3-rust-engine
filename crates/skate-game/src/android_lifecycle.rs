//! Android lifecycle: silence audio and stop the simulation while the app is in the background.
use bevy::{prelude::*, window::AppLifecycle};

/// True between `WillSuspend` and `WillResume`.
#[derive(Resource, Default)]
pub(crate) struct Suspended(pub bool);

/// Set when this module paused virtual time, so it only undoes its own pause.
#[derive(Default)]
struct Held(bool);

pub(crate) struct AndroidLifecyclePlugin;
impl Plugin for AndroidLifecyclePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Suspended>().add_systems(PreUpdate, lifecycle);
    }
}

fn lifecycle(
    mut events: MessageReader<AppLifecycle>,
    mut suspended: ResMut<Suspended>,
    mut time: ResMut<Time<Virtual>>,
    sinks: Query<&AudioSink>,
    mut held: Local<Held>,
) {
    for event in events.read() {
        let now = match event {
            AppLifecycle::WillSuspend | AppLifecycle::Suspended => true,
            AppLifecycle::WillResume | AppLifecycle::Running => false,
            AppLifecycle::Idle => continue,
        };
        if suspended.0 == now {
            continue;
        }
        suspended.0 = now;
        info!("App lifecycle: {event:?}");
        if now {
            for sink in &sinks {
                sink.pause();
            }
            if !time.is_paused() {
                time.pause();
                held.0 = true;
            }
        } else if std::mem::take(&mut held.0) {
            time.unpause();
        }
    }
}
