use crate::aircraft::attitude::{Attitude, AttitudeState};
use bevy::prelude::*;

#[derive(Component)]
pub struct Bank;
#[derive(Component)]
pub struct Pitch;
/// Screen coordinates: clockwise positive. The outside world banks opposite the aircraft;
/// nose-up moves the horizon down, along the banked instrument's local vertical axis.
pub fn instrument(a: Attitude) -> (f32, f32) {
    (-a.roll.to_radians(), a.pitch * 2.5)
}

pub fn spawn(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                width: px(224),
                height: px(224),
                position_type: PositionType::Relative,
                overflow: Overflow::clip(),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(2)),
                ..default()
            },
            BorderColor::all(Color::srgb(0.25, 0.42, 0.52)),
            BackgroundColor(Color::srgb(0.04, 0.07, 0.10)),
        ))
        .with_children(|p| {
            p.spawn((
                Bank,
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(50),
                    top: percent(50),
                    width: px(0),
                    height: px(0),
                    ..default()
                },
                UiTransform::default(),
            ))
            .with_children(|b| {
                b.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(-2),
                        top: px(-98),
                        width: px(4),
                        height: px(12),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(1., 0.78, 0.19)),
                ));
                b.spawn((
                    Pitch,
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(-600),
                        top: px(-600),
                        width: px(1200),
                        height: px(1200),
                        ..default()
                    },
                    UiTransform::default(),
                ))
                .with_children(|plate| {
                    plate.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            top: px(0),
                            width: percent(100),
                            height: px(600),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.08, 0.38, 0.60)),
                    ));
                    plate.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            top: px(600),
                            width: percent(100),
                            height: px(600),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.36, 0.20, 0.12)),
                    ));
                    plate.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            top: px(599),
                            width: percent(100),
                            height: px(2),
                            ..default()
                        },
                        BackgroundColor(Color::WHITE),
                    ));
                    for angle in (-80_i32..=80).step_by(10).filter(|v| *v != 0) {
                        let y = 600. - angle as f32 * 2.5;
                        let width = if angle % 20 == 0 { 64. } else { 36. };
                        plate.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(600. - width / 2.),
                                top: px(y),
                                width: px(width),
                                height: px(1),
                                ..default()
                            },
                            BackgroundColor(Color::WHITE),
                        ));
                        plate.spawn((
                            Text::new(format!("{}", angle.abs())),
                            super::font(11.),
                            TextColor(Color::WHITE),
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(638),
                                top: px(y - 7.),
                                ..default()
                            },
                        ));
                    }
                });
            });
            // Fixed aircraft reference symbol, above the moving sky/ground.
            for (x, y, w, h) in [
                (52., 110., 45., 3.),
                (123., 110., 45., 3.),
                (94., 110., 3., 12.),
                (123., 110., 3., 12.),
                (108., 108., 6., 6.),
            ] {
                p.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(x),
                        top: px(y),
                        width: px(w),
                        height: px(h),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(1., 0.78, 0.19)),
                ));
            }
            // Stationary bank scale; the rotated short pointer belongs to Bank.
            for deg in [-60_f32, -30., 0., 30., 60.] {
                let rad = deg.to_radians();
                let x = 110. + rad.sin() * 98.;
                let y = 110. - rad.cos() * 98.;
                p.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(x - 1.),
                        top: px(y - 5.),
                        width: px(2),
                        height: px(10),
                        ..default()
                    },
                    UiTransform::from_rotation(Rot2::radians(deg.to_radians())),
                    BackgroundColor(Color::WHITE),
                ));
            }
        });
    parent.spawn((
        Text::new("BANK  -60   -30   0   +30   +60"),
        super::font(11.),
        TextColor(super::MUTED),
    ));
}
pub fn update(
    state: Res<AttitudeState>,
    mut bank: Query<&mut UiTransform, With<Bank>>,
    mut pitch: Query<&mut UiTransform, (With<Pitch>, Without<Bank>)>,
) {
    let (r, y) = instrument(state.displayed_angles());
    for mut t in &mut bank {
        t.rotation = Rot2::radians(r);
    }
    for mut t in &mut pitch {
        t.translation = Val2::px(0., y);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observer_moves_opposite_body() {
        let (r, y) = instrument(Attitude {
            roll: 30.,
            pitch: 20.,
        });
        assert!(r < 0.);
        assert_eq!(y, 50.);
        assert_eq!(instrument(Attitude::default()), (0., 0.));
        let (r, y) = instrument(Attitude {
            roll: -30.,
            pitch: -20.,
        });
        assert!(r > 0.);
        assert_eq!(y, -50.);
    }
}
