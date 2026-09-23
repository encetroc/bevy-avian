use avian3d::prelude::{ColliderDensity, Friction, Restitution};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// A gameplay material preset, intentionally separate from Avian physics components.
#[derive(Component, Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum GameplayMaterial {
    Wood,
    Ceramic,
    Stone,
    Metal,
    Glass,
}

/// Gameplay-only resistance to breaking under an impact, in application-defined force units.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct BreakThreshold(pub f32);

/// Gameplay-only multiplier for forces produced by wind systems.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct WindResponse(pub f32);

/// Tunable defaults associated with one gameplay material preset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GameplayMaterialDefaults {
    pub density: f32,
    pub friction: f32,
    pub restitution: f32,
    pub break_threshold: f32,
    pub wind_response: f32,
}

impl GameplayMaterial {
    pub const ALL: [Self; 5] = [
        Self::Wood,
        Self::Ceramic,
        Self::Stone,
        Self::Metal,
        Self::Glass,
    ];

    pub const fn defaults(self) -> GameplayMaterialDefaults {
        match self {
            Self::Wood => GameplayMaterialDefaults {
                density: 0.7,
                friction: 0.65,
                restitution: 0.15,
                break_threshold: 35.0,
                wind_response: 1.0,
            },
            Self::Ceramic => GameplayMaterialDefaults {
                density: 2.4,
                friction: 0.45,
                restitution: 0.1,
                break_threshold: 18.0,
                wind_response: 0.25,
            },
            Self::Stone => GameplayMaterialDefaults {
                density: 2.6,
                friction: 0.8,
                restitution: 0.05,
                break_threshold: 90.0,
                wind_response: 0.1,
            },
            Self::Metal => GameplayMaterialDefaults {
                density: 7.8,
                friction: 0.35,
                restitution: 0.2,
                break_threshold: 120.0,
                wind_response: 0.05,
            },
            Self::Glass => GameplayMaterialDefaults {
                density: 2.5,
                friction: 0.3,
                restitution: 0.25,
                break_threshold: 8.0,
                wind_response: 0.2,
            },
        }
    }
}

/// Applies defaults to newly assigned materials.
///
/// Avian remains directly usable: physics properties are ordinary Avian components that can be
/// tuned without going through `GameplayMaterial`. The preset takes precedence over Avian's
/// automatically-required component defaults at assignment time; later edits to those Avian
/// components are independent.
pub struct GameplayMaterialsPlugin;

impl Plugin for GameplayMaterialsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, apply_gameplay_material_defaults);
    }
}

fn apply_gameplay_material_defaults(
    mut commands: Commands,
    entities: Query<(Entity, &GameplayMaterial), Added<GameplayMaterial>>,
) {
    for (entity, material) in &entities {
        let defaults = material.defaults();
        commands.entity(entity).insert((
            ColliderDensity(defaults.density),
            Friction::new(defaults.friction),
            Restitution::new(defaults.restitution),
            BreakThreshold(defaults.break_threshold),
            WindResponse(defaults.wind_response),
        ));
    }
}

#[cfg(test)]
mod tests {
    use avian3d::prelude::Collider;

    use super::*;

    #[test]
    fn identical_bodies_receive_distinct_defaults_and_keep_physics_separate() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, GameplayMaterialsPlugin));

        let wood = app
            .world_mut()
            .spawn((GameplayMaterial::Wood, Collider::cuboid(1.0, 1.0, 1.0)))
            .id();
        let glass = app
            .world_mut()
            .spawn((GameplayMaterial::Glass, Collider::cuboid(1.0, 1.0, 1.0)))
            .id();
        app.update();

        let wood_entity = app.world().entity(wood);
        let wood_defaults = GameplayMaterial::Wood.defaults();
        assert_eq!(
            wood_entity.get::<ColliderDensity>(),
            Some(&ColliderDensity(wood_defaults.density))
        );
        assert_eq!(
            wood_entity.get::<Friction>().unwrap().dynamic_coefficient,
            wood_defaults.friction
        );
        assert_eq!(
            wood_entity.get::<Restitution>().unwrap().coefficient,
            wood_defaults.restitution
        );
        assert_eq!(
            wood_entity.get::<BreakThreshold>(),
            Some(&BreakThreshold(wood_defaults.break_threshold))
        );
        assert_eq!(
            wood_entity.get::<WindResponse>(),
            Some(&WindResponse(wood_defaults.wind_response))
        );

        let glass_entity = app.world().entity(glass);
        let glass_defaults = GameplayMaterial::Glass.defaults();
        assert_eq!(
            glass_entity.get::<ColliderDensity>(),
            Some(&ColliderDensity(glass_defaults.density))
        );
        assert_eq!(
            glass_entity.get::<Friction>().unwrap().dynamic_coefficient,
            glass_defaults.friction
        );
        assert_eq!(
            glass_entity.get::<Restitution>().unwrap().coefficient,
            glass_defaults.restitution
        );
        assert_eq!(
            glass_entity.get::<BreakThreshold>(),
            Some(&BreakThreshold(glass_defaults.break_threshold))
        );
        assert_eq!(
            glass_entity.get::<WindResponse>(),
            Some(&WindResponse(glass_defaults.wind_response))
        );
        assert_ne!(
            wood_entity.get::<BreakThreshold>(),
            glass_entity.get::<BreakThreshold>()
        );

        // Direct Avian edits remain independent after the preset is first applied.
        app.world_mut()
            .entity_mut(glass)
            .insert(ColliderDensity(4.0));
        app.update();
        assert_eq!(
            app.world().entity(glass).get::<ColliderDensity>(),
            Some(&ColliderDensity(4.0))
        );
        assert_eq!(
            app.world().entity(glass).get::<BreakThreshold>(),
            Some(&BreakThreshold(glass_defaults.break_threshold))
        );
    }

    #[test]
    fn all_presets_have_physical_and_gameplay_defaults() {
        for material in GameplayMaterial::ALL {
            let defaults = material.defaults();
            assert!(defaults.density > 0.0);
            assert!(defaults.friction >= 0.0);
            assert!((0.0..=1.0).contains(&defaults.restitution));
            assert!(defaults.break_threshold > 0.0);
            assert!(defaults.wind_response >= 0.0);
        }
    }
}
