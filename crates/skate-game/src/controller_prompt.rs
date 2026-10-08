//! Android only: "Connect your controller" text while no pad slot is ready.
use crate::input::ControllerInput;
use bevy::prelude::*;

pub(crate) struct ControllerPromptPlugin;

impl Plugin for ControllerPromptPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update);
    }
}

#[derive(Component)]
struct Prompt;

fn update(
    mut commands: Commands,
    input: Option<Res<ControllerInput>>,
    cameras: Query<Entity, With<IsDefaultUiCamera>>,
    mut prompt: Query<&mut Visibility, With<Prompt>>,
) {
    let Some(input) = input else { return };
    let want = if input.active_slot().is_none() { Visibility::Inherited } else { Visibility::Hidden };
    if let Ok(mut vis) = prompt.single_mut() {
        vis.set_if_neq(want);
        return;
    }
    let Ok(output) = cameras.single() else { return };
    commands.spawn((
        Prompt,
        want,
        UiTargetCamera(output),
        GlobalZIndex(20),
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        children![(
            Text::new("Connect your controller"),
            TextFont { font_size: 36., ..default() },
            TextColor(Color::WHITE),
            BackgroundColor(Color::srgba(0., 0., 0., 0.6)),
            Node { padding: UiRect::axes(px(24), px(12)), ..default() },
        )],
    ));
}
