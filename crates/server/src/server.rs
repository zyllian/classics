pub mod config;
pub(crate) mod network;

use std::{path::PathBuf, sync::Arc};

use rand::{seq::SliceRandom, Rng};
use tokio::{net::TcpListener, sync::RwLock};

use crate::CONFIG_FILE;
use internal::{
	error::GeneralError,
	level::{
		block::{
			BlockType, BLOCK_INFO, ID_DIRT, ID_GRASS, ID_LAVA_FLOWING, ID_LAVA_STATIONARY,
			ID_STONE, ID_WATER_FLOWING, ID_WATER_STATIONARY,
		},
		BlockUpdate, Level,
	},
	packet::server::ServerPacket,
	player::Player,
	util::{
		get_relative_coords, neighbors_full, neighbors_minus_up, neighbors_with_vertical_diagonals,
	},
};

use self::config::ServerConfig;

const TICK_DURATION: std::time::Duration = std::time::Duration::from_millis(50);
const LEVELS_PATH: &str = "levels";

/// the server
#[derive(Debug)]
pub struct Server {
	/// shared server data
	pub data: Arc<RwLock<ServerData>>,
	/// the server's listener
	pub listener: TcpListener,
}

/// shared server data
#[derive(Debug)]
pub struct ServerData {
	/// the level
	pub level: Level,
	/// list of players connected to the server
	pub players: Vec<Player>,
	/// list of player ids which have been freed up
	pub free_player_ids: Vec<i8>,
	/// the server's config
	pub config: ServerConfig,
	/// whether the server config needs to be resaved or not
	pub config_needs_saving: bool,
	/// whether the server should be stopped
	pub stop: bool,
}

impl ServerData {
	/// spreads a packet to all players
	pub fn spread_packet(&mut self, packet: ServerPacket) {
		for player in &mut self.players {
			player.packets_to_send.push(packet.clone());
		}
	}

	/// spreads multiple packets to all players
	pub fn spread_packets(&mut self, packets: &[ServerPacket]) {
		for player in &mut self.players {
			for packet in packets {
				player.packets_to_send.push(packet.clone());
			}
		}
	}
}

impl Server {
	/// creates a new server with a generated level
	pub async fn new(config: ServerConfig) -> Result<Self, GeneralError> {
		let levels_path = PathBuf::from(LEVELS_PATH);
		if !levels_path.exists() {
			std::fs::create_dir_all(&levels_path)?;
		}
		let level_path = levels_path.join(&config.level_name);
		let level = if level_path.exists() {
			Level::load(level_path).await?
		} else {
			println!("generating level");
			let mut rng = rand::thread_rng();
			let mut level = Level::new(
				config.level_size.x,
				config.level_size.y,
				config.level_size.z,
			);
			config.generation.generate(&mut level, &mut rng);
			level.save(level_path).await?;
			println!("done!");
			level
		};

		Self::new_with_level(config, level).await
	}

	/// creates a new server with the given level
	pub async fn new_with_level(config: ServerConfig, level: Level) -> Result<Self, GeneralError> {
		let listener = TcpListener::bind("0.0.0.0:25565").await?;

		Ok(Self {
			data: Arc::new(RwLock::new(ServerData {
				level,
				players: Default::default(),
				free_player_ids: Vec::new(),
				config,
				config_needs_saving: true,
				stop: false,
			})),
			listener,
		})
	}

	/// starts the server
	pub async fn run(self) -> Result<(), GeneralError> {
		let data = self.data.clone();
		tokio::spawn(async move {
			loop {
				let (stream, addr) = self
					.listener
					.accept()
					.await
					.expect("failed to accept listener!");
				println!("connection from {addr}");
				let data = data.clone();
				tokio::spawn(async move {
					network::handle_stream(stream, addr, data).await;
				});
			}
		});
		println!("server is started!");
		handle_ticks(self.data.clone()).await?;
		tokio::time::sleep(std::time::Duration::from_millis(1)).await;

		// TODO: cancel pending tasks/send out "Server is stopping" messages *here* instead of elsewhere
		// rn the message isn't guaranteed to actually go out........

		let mut data = self.data.write().await;
		let player_data = data
			.players
			.iter()
			.map(|p| (p.username.clone(), p.savable_data.clone()))
			.collect();
		data.level.update_player_data(player_data);
		data.level
			.save(PathBuf::from(LEVELS_PATH).join(&data.config.level_name))
			.await?;

		Ok(())
	}
}

/// function to tick the server
async fn handle_ticks(data: Arc<RwLock<ServerData>>) -> Result<(), GeneralError> {
	let mut current_tick = 0;
	let mut last_auto_save = std::time::Instant::now();
	loop {
		{
			let mut data = data.write().await;
			tick(&mut data, current_tick);

			if data.config_needs_saving {
				tokio::fs::write(CONFIG_FILE, serde_json::to_string_pretty(&data.config)?).await?;
				data.config_needs_saving = false;
			}

			if data.stop {
				let packet = ServerPacket::DisconnectPlayer {
					disconnect_reason: "Server is stopping!".to_string(),
				};
				for player in &mut data.players {
					player.packets_to_send.push(packet.clone());
				}
				break;
			}

			if data.level.save_now
				|| (data.config.auto_save_minutes != 0
					&& last_auto_save.elapsed().as_secs() / 60 >= data.config.auto_save_minutes)
			{
				data.level.save_now = false;
				data.level
					.save(PathBuf::from(LEVELS_PATH).join(&data.config.level_name))
					.await?;
				last_auto_save = std::time::Instant::now();

				let packet = ServerPacket::Message {
					player_id: -1,
					message: "Server has saved!".to_string(),
				};
				for player in &mut data.players {
					player.packets_to_send.push(packet.clone());
				}
			}
		}

		current_tick = current_tick.wrapping_add(1);
		tokio::time::sleep(TICK_DURATION).await;
	}

	Ok(())
}

/// function which ticks the server once
fn tick(data: &mut ServerData, tick: usize) {
	let level = &mut data.level;

	let mut packets = level.apply_updates();

	// apply random tick updates
	let mut rng = rand::thread_rng();
	level.possible_random_updates.shuffle(&mut rng);
	for _ in 0..level.rules.random_tick_updates {
		if let Some(index) = level.possible_random_updates.pop() {
			level.awaiting_update.insert(index);
		} else {
			break;
		}
	}

	let awaiting_update = std::mem::take(&mut level.awaiting_update);
	for index in awaiting_update {
		let (x, y, z) = level.coordinates(index);
		let block_id = level.get_block(x, y, z);
		let block = BLOCK_INFO.get(&block_id).expect("should never fail");
		match &block.block_type {
			BlockType::Solid => {
				if block_id == ID_GRASS {
					let mut dirt_count = 0;
					for (nx, ny, nz) in neighbors_with_vertical_diagonals(level, x, y, z) {
						if level.get_block(nx, ny, nz) == ID_DIRT {
							// only turn dirt into grass if there's empty space above it
							if get_relative_coords(level, nx, ny, nz, 0, 1, 0)
								.map(|(x, y, z)| level.get_block(x, y, z))
								.is_none_or(|id| id == 0x00)
							{
								dirt_count += 1;
								if rng.gen_range(0..level.rules.grass_spread_chance) == 0 {
									dirt_count -= 1;
									level.updates.push(BlockUpdate {
										index: level.index(nx, ny, nz),
										block: ID_GRASS,
									});
								}
							}
						}
					}
					if get_relative_coords(level, x, y, z, 0, 1, 0)
						.map(|(x, y, z)| level.get_block(x, y, z))
						.is_some_and(|id| id != 0x00)
					{
						dirt_count += 1;
						if rng.gen_range(0..level.rules.grass_spread_chance) == 0 {
							dirt_count -= 1;
							level.updates.push(BlockUpdate {
								index: level.index(x, y, z),
								block: ID_DIRT,
							});
						}
					}
					if dirt_count > 0 {
						level.possible_random_updates.push(level.index(x, y, z));
					}
				} else if block_id == ID_DIRT {
					for (nx, ny, nz) in neighbors_full(level, x, y, z) {
						if level.get_block(nx, ny, nz) == ID_GRASS {
							level.possible_random_updates.push(level.index(nx, ny, nz));
						}
					}
				}
			}
			BlockType::FluidFlowing {
				stationary,
				ticks_to_spread,
			} => {
				if !level.rules.fluid_spread {
					continue;
				}
				if tick % ticks_to_spread == 0 {
					let update = BlockUpdate {
						index,
						block: *stationary,
					};
					level.updates.push(update);
					for (nx, ny, nz) in neighbors_minus_up(level, x, y, z) {
						let id = level.get_block(nx, ny, nz);
						let block_at = BLOCK_INFO.get(&id).expect("missing block");
						let index = level.index(nx, ny, nz);
						let update = match block_at.block_type {
							BlockType::NonSolid => BlockUpdate {
								index,
								block: block_id,
							},
							BlockType::FluidFlowing { .. } | BlockType::FluidStationary { .. } => {
								let turn_to_stone = match block_id {
									ID_WATER_FLOWING | ID_WATER_STATIONARY => {
										id == ID_LAVA_FLOWING || id == ID_LAVA_STATIONARY
									}
									ID_LAVA_FLOWING | ID_LAVA_STATIONARY => {
										id == ID_WATER_FLOWING || id == ID_WATER_STATIONARY
									}
									_ => panic!(
										"unimplemented fluid interactions for fluid: {}",
										block.str_id
									),
								};
								if turn_to_stone {
									BlockUpdate {
										index,
										block: ID_STONE,
									}
								} else {
									continue;
								}
							}
							_ => continue,
						};
						level.awaiting_update.insert(index);
						level.updates.push(update);
					}
				} else {
					level.awaiting_update.insert(index);
				}
			}
			BlockType::FluidStationary { moving } => {
				if !level.rules.fluid_spread {
					continue;
				}
				let mut needs_update = false;
				for (nx, ny, nz) in neighbors_minus_up(level, x, y, z) {
					if matches!(
						BLOCK_INFO
							.get(&level.get_block(nx, ny, nz))
							.expect("missing block")
							.block_type,
						BlockType::NonSolid
					) {
						needs_update = true;
						break;
					}
				}
				if needs_update {
					let index = level.index(x, y, z);
					level.updates.push(BlockUpdate {
						index,
						block: *moving,
					});
					level.awaiting_update.insert(index);
				}
			}
			_ => {}
		}
	}

	packets.extend(level.apply_updates());
	for packet in packets {
		for player in &mut data.players {
			player.packets_to_send.push(packet.clone());
		}
	}
}
