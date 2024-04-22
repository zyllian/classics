pub mod config;
mod network;

use std::sync::Arc;

use tokio::{net::TcpListener, sync::RwLock};

use crate::{
	level::{
		block::{BlockType, BLOCK_INFO},
		BlockUpdate, Level,
	},
	player::Player,
	util::neighbors_minus_up,
};

use self::config::ServerConfig;

const TICK_DURATION: std::time::Duration = std::time::Duration::from_millis(50);

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
}

impl Server {
	/// creates a new server with a generated level
	pub async fn new(config: ServerConfig) -> std::io::Result<Self> {
		println!("generating level");
		let mut rng = rand::thread_rng();
		let mut level = Level::new(
			config.level_size.x,
			config.level_size.y,
			config.level_size.z,
		);
		config.generation.generate(&mut level, &mut rng);
		println!("done!");

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
			})),
			listener,
		})
	}

	/// starts the server
	pub async fn run(&mut self) -> std::io::Result<()> {
		let data = self.data.clone();
		tokio::spawn(async move {
			handle_ticks(data).await;
		});
		loop {
			let (stream, addr) = self.listener.accept().await?;
			println!("connection from {addr}");
			let data = self.data.clone();
			tokio::spawn(async move {
				network::handle_stream(stream, addr, data)
					.await
					.expect("failed to handle client stream");
			});
		}
	}
}

/// function to tick the server
async fn handle_ticks(data: Arc<RwLock<ServerData>>) {
	let mut current_tick = 0;
	loop {
		tick(&mut *data.write().await, current_tick);
		current_tick = current_tick.wrapping_add(1);
		tokio::time::sleep(TICK_DURATION).await;
	}
}

/// function which ticks the server once
fn tick(data: &mut ServerData, tick: usize) {
	let level = &mut data.level;

	let mut packets = level.apply_updates();

	let awaiting_update = std::mem::take(&mut level.awaiting_update);
	if !awaiting_update.is_empty() {
		println!("hm");
	}
	for index in awaiting_update {
		let (x, y, z) = level.coordinates(index);
		let block_id = level.get_block(x, y, z);
		let block = BLOCK_INFO.get(&block_id).expect("should never fail");
		println!("{block:#?}");
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
						let block_at = level.get_block(nx, ny, nz);
						let update = if block_at == 0 {
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
					if level.get_block(nx, ny, nz) == 0 {
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
