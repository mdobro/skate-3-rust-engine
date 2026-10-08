//! "Shadow quality" graphics setting.
//!
//! High is exactly the authored setup (4 cascades to 100 m, 2048 map). Lower
//! tiers cut cascades, distance and map size. Off disables shadow rendering on
//! every shadow light; the world and character shaders only sample lights whose
//! shadow flag is set, so both fall back to unshadowed lighting.
use bevy::light::{CascadeShadowConfig, CascadeShadowConfigBuilder, DirectionalLightShadowMap};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ShadowQuality {
    Off,
    Low,
    Medium,
    #[default]
    High,
}

impl ShadowQuality {
    pub(crate) const ALL: [Self; 4] = [Self::Off, Self::Low, Self::Medium, Self::High];

    /// First-run default: Medium on Android, High (unchanged) elsewhere.
    pub(crate) fn platform_default() -> Self {
        if cfg!(target_os = "android") { Self::Medium } else { Self::High }
    }
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
        }
    }
    fn map_size(self) -> usize {
        match self {
            Self::Low => 1024,
            _ => 2048,
        }
    }
}

/// Which shadow light an entity is, so the tiers can reshape each differently.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShadowTier {
    /// Scene sun: all-caster visibility sampled by the character shader.
    Scene,
    /// Layer-28 receiver-only light for dynamic shadows on the baked world.
    Receiver,
    /// Fallback scene light (`world.rs`).
    Fallback,
}

impl ShadowTier {
    fn config(self, quality: ShadowQuality) -> CascadeShadowConfig {
        use ShadowQuality::*;
        let builder = match (self, quality) {
            (Self::Scene, High | Off) => CascadeShadowConfigBuilder {
                maximum_distance: 100.,
                first_cascade_far_bound: 10.,
                ..default()
            },
            (Self::Fallback, High | Off) => CascadeShadowConfigBuilder::default(),
            (Self::Receiver, High | Medium | Off) => CascadeShadowConfigBuilder {
                num_cascades: 1,
                maximum_distance: 24.,
                ..default()
            },
            (Self::Receiver, Low) => CascadeShadowConfigBuilder {
                num_cascades: 1,
                maximum_distance: 16.,
                ..default()
            },
            (_, Medium) => CascadeShadowConfigBuilder {
                num_cascades: 2,
                maximum_distance: 60.,
                first_cascade_far_bound: 10.,
                ..default()
            },
            (_, Low) => CascadeShadowConfigBuilder {
                num_cascades: 1,
                maximum_distance: 30.,
                ..default()
            },
        };
        builder.build()
    }
}

/// Reapplies the setting when it changes or a shadow light appears (lights are
/// respawned on map changes).
pub(crate) fn apply(
    menu: Option<Res<crate::graphics_menu::Menu>>,
    mut map: ResMut<DirectionalLightShadowMap>,
    mut lights: Query<(&ShadowTier, &mut DirectionalLight, &mut CascadeShadowConfig)>,
    added: Query<(), Added<ShadowTier>>,
    mut last: Local<Option<ShadowQuality>>,
) {
    let quality = menu.map_or_else(ShadowQuality::platform_default, |m| m.shadow_quality());
    if *last == Some(quality) && added.is_empty() {
        return;
    }
    if *last != Some(quality) {
        info!("Shadow quality {}", quality.label());
        if map.size != quality.map_size() {
            map.size = quality.map_size();
        }
    }
    *last = Some(quality);
    for (tier, mut light, mut cascades) in &mut lights {
        let enabled = quality != ShadowQuality::Off;
        if light.shadows_enabled != enabled {
            light.shadows_enabled = enabled;
        }
        *cascades = tier.config(quality);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_matches_authored_setup() {
        let scene = ShadowTier::Scene.config(ShadowQuality::High);
        assert_eq!(scene.bounds.len(), 4);
        assert_eq!(*scene.bounds.last().unwrap(), 100.);
        let receiver = ShadowTier::Receiver.config(ShadowQuality::High);
        assert_eq!(receiver.bounds, vec![24.]);
        assert_eq!(ShadowQuality::High.map_size(), 2048);
    }

    #[test]
    fn lower_tiers_reduce_cost() {
        let medium = ShadowTier::Scene.config(ShadowQuality::Medium);
        let low = ShadowTier::Scene.config(ShadowQuality::Low);
        assert_eq!(medium.bounds.len(), 2);
        assert_eq!(low.bounds.len(), 1);
        assert!(ShadowQuality::Low.map_size() < ShadowQuality::Medium.map_size());
    }

    #[test]
    fn old_settings_default_to_platform() {
        assert_eq!(ShadowQuality::default(), ShadowQuality::High);
    }
}
