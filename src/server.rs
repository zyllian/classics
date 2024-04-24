pub mod config;
mod network;

use std::{path::PathBuf, sync::Arc};

use tokio::{net::TcpListener, sync::RwLock};

use crate::{
	level::{
		block::{BlockType, BLOCK_INFO},
		BlockUpdate, Level,
	},
	packet::server::ServerPacket,
	player::Player,
	util::neighbors_minus_up,
	CONFIG_FILE,
};

use self::config::ServerConfig;

const TICK_DURATION: std::time::Duration = std::time::Duration::from_millis(50);
const LEVEL_PATH: &str = "level.clw";

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
}

impl Server {
	/// creates a new server with a generated level
	pub async fn new(config: ServerConfig) -> std::io::Result<Self> {
		let level_path = PathBuf::from(LEVEL_PATH);
		let level = if level_path.exists() {
			Level::load(level_path).await
		} else {
			println!("generating level");
			let mut rng = rand::thread_rng();
			let mut level = Level::new(
				config.level_size.x,
				config.level_size.y,
				config.level_size.z,
			);
			config.generation.generate(&mut level, &mut rng);
			println!("done!");
			level
		};

		Self::new_with_level(config, level).await
	}

	/// creates a new server with the given level
	pub async fn new_with_level(config: ServerConfig, level: Level) -> std::io::Result<Self> {
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
	pub async fn run(self) -> std::io::Result<()> {
		let data = self.data.clone();
		tokio::spawn(async move {
			loop {
				let (stream, addr) = self.listener.accept().await.unwrap();
				println!("connection from {addr}");
				let data = data.clone();
				tokio::spawn(async move {
					network::handle_stream(stream, addr, data)
						.await
						.expect("failed to handle client stream");
				});
			}
		});
		handle_ticks(self.data.clone()).await;
		tokio::time::sleep(std::time::Duration::from_millis(1)).await;

		// TODO: cancel pending tasks/send out "Server is stopping" messages *here* instead of elsewhere
		// rn the message isn't guaranteed to actually go out........

		self.data.read().await.level.save(LEVEL_PATH).await;

		Ok(())
	}
}

/// function to tick the server
async fn handle_ticks(data: Arc<RwLock<ServerData>>) {
	let mut current_tick = 0;
	let mut last_auto_save = std::time::Instant::now();
	loop {
		{
			let mut data = data.write().await;
			tick(&mut data, current_tick);

			if data.config_needs_saving {
				std::fs::write(
					CONFIG_FILE,
					serde_json::to_string_pretty(&data.config)
						.expect("failed to serialize default config"),
				)
				.expect("failed to save config file");
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

			if data.config.auto_save_minutes != 0
				&& last_auto_save.elapsed().as_secs() / 60 >= data.config.auto_save_minutes
			{
				data.level.save(LEVEL_PATH).await;
				last_auto_save = std::time::Instant::now();
			}
		}

		current_tick = current_tick.wrapping_add(1);
		tokio::time::sleep(TICK_DURATION).await;
	}
}

/// function which ticks the server once
fn tick(data: &mut ServerData, tick: usize) {
	let level = &mut data.level;

	let mut packets = level.apply_updates();

	let awaiting_update = std::mem::take(&mut level.awaiting_update);
	for index in awaiting_update {
		let (x, y, z) = level.coordinates(index);
		let block_id = level.get_block(x, y, z);
		let block = BLOCK_INFO.get(&block_id).expect("should never fail");
		match &block.block_type {
			BlockType::FluidFlowing {
				stationary,
				ticks_to_spread,
			} => {
				if tick % ticks_to_spread == 0 {
					let update = BlockUpdate {
						index,
						block: *stationary,
					};
					level.updates.push(update);
					for (nx, ny, nz) in neighbors_minus_up(level, x, y, z) {
						let block_at = BLOCK_INFO
							.get(&level.get_block(nx, ny, nz))
							.expect("missing block");
						let update = if matches!(block_at.block_type, BlockType::NonSolid) {
							level.awaiting_update.insert(level.index(nx, ny, nz));
							BlockUpdate {
								index: level.index(nx, ny, nz),
								block: block_id,
							}
						} else {
							continue;
						};
						level.updates.push(update);
					}
				} else {
					level.awaiting_update.insert(index);
				}
			}
			BlockType::FluidStationary { moving } => {
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
