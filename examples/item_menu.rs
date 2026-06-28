//! This example shows how to use a read-only [`OpenInventory`] as a menu,
//! in which the player is able to select items by clicking on them.
//! This is commonly used on minigame servers (e.g for team selection).
#[allow(unused_imports)]
use chunkedge::inventory::ClickSlotMessage;
use chunkedge::log::LogPlugin;
use chunkedge::prelude::*;
use chunkedge::protocol::sound::SoundCategory;
use chunkedge::protocol::Sound;
use std::marker::PhantomData;

const SPAWN_Y: i32 = 64;

fn main() {
    App::new()
        .insert_resource(NetworkSettings {
            connection_mode: ConnectionMode::Offline,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins.build().disable::<LogPlugin>())
        .add_plugins(InventoryMenuPlugin::<RedGreenClick>::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (init_clients, despawn_disconnected_clients, sneak))
        .add_systems(Update, click_in_menu)
        .run();
}

fn setup(
    mut commands: Commands,
    server: Res<Server>,
    biomes: Res<BiomeRegistry>,
    dimensions: Res<DimensionTypeRegistry>,
) {
    let mut layer = LayerBundle::new(ident!("overworld"), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            layer.chunk.insert_chunk([x, z], UnloadedChunk::new());
        }
    }

    for z in -25..25 {
        for x in -25..25 {
            layer
                .chunk
                .set_block([x, SPAWN_Y, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(layer);
}

fn init_clients(
    mut clients: Query<
        (
            &mut EntityLayerId,
            &mut VisibleChunkLayer,
            &mut VisibleEntityLayers,
            &mut Position,
            &mut GameMode,
        ),
        Added<Client>,
    >,
    layers: Query<Entity, (With<ChunkLayer>, With<EntityLayer>)>,
) {
    for (
        mut layer_id,
        mut visible_chunk_layer,
        mut visible_entity_layers,
        mut pos,
        mut game_mode,
    ) in &mut clients
    {
        let layer = layers.single().unwrap();

        layer_id.0 = layer;
        visible_chunk_layer.0 = layer;
        visible_entity_layers.0.insert(layer);
        pos.set([0.0, f64::from(SPAWN_Y) + 1.0, 0.0]);
        *game_mode = GameMode::Creative;
    }
}

fn sneak(
    clients: Query<(Entity, &Client)>,
    mut messages: MessageReader<SneakMessage>,
    mut commands: Commands,
) {
    for message in messages.read() {
        if message.state == SneakState::Start {
            if let Ok((entity, _)) = clients.get(message.client) {
                open_menu(&mut commands, entity)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum RedGreenClick {
    Red,
    Green,
}

fn open_menu(commands: &mut Commands, player: Entity) {
    let mut menu_inv = Inventory::new(InventoryKind::Generic3x3);
    menu_inv.readonly = true;

    menu_inv.set_slot(
        3,
        ItemStack::new_with_entity(
            ItemKind::RedWool,
            1,
            commands
                .spawn(OnClicked {
                    action: RedGreenClick::Red,
                })
                .id(),
        ),
    );
    menu_inv.set_slot(
        5,
        ItemStack::new_with_entity(
            ItemKind::GreenWool,
            1,
            commands
                .spawn(OnClicked {
                    action: RedGreenClick::Green,
                })
                .id(),
        ),
    );

    let inventory = commands.spawn(menu_inv).id();

    commands
        .entity(player)
        .insert(OpenInventory::new(inventory));
}

fn click_in_menu(
    mut messages: MessageReader<OnClickedSelectMessage<RedGreenClick>>,
    mut clients: Query<(&mut Client, &Position)>,
) {
    for message in messages.read() {
        let Ok((mut client, pos)) = clients.get_mut(message.client) else {
            continue;
        };

        client.play_sound(
            Sound::BlockNoteBlockBit,
            SoundCategory::Block,
            pos.0,
            1.0,
            1.0,
        );

        match message.action {
            RedGreenClick::Red => {
                client.send_chat_message("You clicked §cRed");
            }
            RedGreenClick::Green => {
                client.send_chat_message("You clicked §aGreen");
            }
        }
    }
}

// on click api stuff below

struct InventoryMenuPlugin<T> {
    _marker: PhantomData<T>,
}

impl<T> Default for InventoryMenuPlugin<T> {
    fn default() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<T: Send + Sync + Clone + 'static> Plugin for InventoryMenuPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_message::<OnClickedSelectMessage<T>>()
            .add_systems(Update, select_menu_item::<T>);
    }
}

#[derive(Component, Debug, Clone)]
struct OnClicked<T> {
    action: T,
}

#[derive(Debug, Clone, PartialEq, Message)]
struct OnClickedSelectMessage<T> {
    /// Player entity
    pub client: Entity,
    /// Clicked item
    pub clicked: ItemStack,
    /// T action
    action: T,
}

fn select_menu_item<T: Send + Sync + Clone + 'static>(
    mut clients: Query<Entity>,
    mut messages: MessageReader<ClickSlotMessage>,
    mut message_writer: MessageWriter<OnClickedSelectMessage<T>>,
    click_query: Query<&OnClicked<T>>,
) {
    for message in messages.read() {
        let Ok(player) = clients.get_mut(message.client) else {
            continue;
        };
        let Some(clicked_item_entity) = message.carried_item.entity else {
            continue;
        };

        let Ok(clicked_item_on_clicked) = click_query.get(clicked_item_entity) else {
            continue;
        };

        message_writer.write(OnClickedSelectMessage {
            client: player,
            clicked: message.carried_item.clone(),
            action: clicked_item_on_clicked.action.clone(),
        });
    }
}
