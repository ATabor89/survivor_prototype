//! Shared generated sprites for inventory icons, world pickups, and projectiles.
use bevy::prelude::*;

#[derive(Component, Clone, Copy)]
pub struct ItemIcon(pub usize);

#[derive(Resource)]
struct ItemSprites {
    image: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
    arsenal: Vec<Handle<Image>>,
}

pub struct ArtPlugin;
impl Plugin for ArtPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_item_sprites)
            .add_systems(PostUpdate, apply_item_art);
    }
}

fn load_item_sprites(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.insert_resource(ItemSprites {
        arsenal: [
            "knife",
            "talismans",
            "inscription",
            "pelican",
            "sigil",
            "familiar",
            "lattice",
            "recitation",
        ]
        .map(|name| assets.load(format!("sprites/arsenal/{name}.png")))
        .to_vec(),
        image: assets.load("sprites/occult-items-v1.png"),
        // Generated output is 1448 x 1086: four columns, three rows.
        layout: layouts.add(TextureAtlasLayout::from_grid(
            UVec2::splat(362),
            4,
            3,
            None,
            None,
        )),
    });
}

#[expect(
    clippy::type_complexity,
    reason = "One sprite marker supports both world sprites and UI image nodes."
)]
fn apply_item_art(
    sprites: Res<ItemSprites>,
    mut icons: Query<(&ItemIcon, Option<&mut Sprite>, Option<&mut ImageNode>), Changed<ItemIcon>>,
) {
    for (icon, sprite, node) in &mut icons {
        let (image, atlas) = match icon
            .0
            .checked_sub(12)
            .and_then(|index| sprites.arsenal.get(index))
        {
            Some(image) => (image.clone(), None),
            None => (
                sprites.image.clone(),
                Some(TextureAtlas {
                    layout: sprites.layout.clone(),
                    index: icon.0.min(11),
                }),
            ),
        };
        if let Some(mut sprite) = sprite {
            sprite.image = image.clone();
            sprite.texture_atlas = atlas.clone();
            sprite.color = Color::WHITE;
        }
        if let Some(mut node) = node {
            node.image = image;
            node.texture_atlas = atlas;
            node.color = Color::WHITE;
        }
    }
}
