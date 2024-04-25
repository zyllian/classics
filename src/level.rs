use std::{
	collections::BTreeSet,
	io::{Read, Write},
	path::Path,
};

use serde::{Deserialize, Serialize};

use crate::{packet::server::ServerPacket, util::neighbors};

use self::block::BLOCK_INFO;

pub mod block;
pub mod generation;

const LEVEL_INFO_PATH: &str = "info.json";
const LEVEL_DATA_PATH: &str = "level.dat";

/// a classic level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Level {
	/// the size of the level in the X direction
	pub x_size: usize,
	/// the size of the level in the Y direction
	pub y_size: usize,
	/// the size of the level in the Z direction
	pub z_size: usize,

	/// the blocks which make up the level
	#[serde(skip)]
	pub blocks: Vec<u8>,
	/// the level's weather
	pub weather: WeatherType,

	/// index of blocks which need to be updated in the next tick
	pub awaiting_update: BTreeSet<usize>,
	/// list of updates to apply to the world on the next tick
	#[serde(skip)]
	pub updates: Vec<BlockUpdate>,
	#[serde(skip)]
	pub save_now: bool,
}

impl Level {
	/// creates a new level with the given dimensions
	pub fn new(x_size: usize, y_size: usize, z_size: usize) -> Self {
		Self {
			x_size,
			y_size,
			z_size,
			blocks: vec![0; x_size * y_size * z_size],
			weather: WeatherType::Sunny,
			awaiting_update: Default::default(),
			updates: Default::default(),
			save_now: false,
		}
	}

	/// gets the index for a given block position
	pub fn index(&self, x: usize, y: usize, z: usize) -> usize {
		x + z * self.x_size + y * self.x_size * self.z_size
	}

	/// gets the coordinates for the given index
	pub fn coordinates(&self, index: usize) -> (usize, usize, usize) {
		let y = index / (self.x_size * self.z_size);
		let z = (index / self.x_size) % self.z_size;
		let x = index % self.z_size;
		(x, y, z)
	}

	/// gets the block at the given position
	pub fn get_block(&self, x: usize, y: usize, z: usize) -> u8 {
		self.blocks[self.index(x, y, z)]
	}

	/// sets the block at the given position
	pub fn set_block(&mut self, x: usize, y: usize, z: usize, block: u8) {
		let index = self.index(x, y, z);
		self.blocks[index] = block;
	}

	/// applies the level's queued updates
	pub fn apply_updates(&mut self) -> Vec<ServerPacket> {
		self.updates.dedup_by(|a, b| a.index == b.index);
		let mut packets = Vec::with_capacity(self.updates.len());

		for update in std::mem::take(&mut self.updates) {
			let (x, y, z) = self.coordinates(update.index);
			self.blocks[update.index] = update.block;
			packets.push(ServerPacket::SetBlock {
				x: x as i16,
				y: y as i16,
				z: z as i16,
				block_type: update.block,
			});
			for (nx, ny, nz) in neighbors(self, x, y, z) {
				let info = BLOCK_INFO
					.get(&self.get_block(nx, ny, nz))
					.expect("missing block");
				if info.block_type.needs_update_when_neighbor_changed() {
					self.awaiting_update.insert(self.index(nx, ny, nz));
				}
			}
		}

		packets
	}

	/// saves the level
	pub async fn save<P>(&self, path: P) -> std::io::Result<()>
	where
		P: AsRef<Path>,
	{
		let path = path.as_ref();
		tokio::fs::create_dir_all(path).await?;
		tokio::fs::write(
			path.join(LEVEL_INFO_PATH),
			serde_json::to_string_pretty(self).unwrap(),
		)
		.await?;
		let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
		encoder
			.write_all(&self.blocks)
			.expect("failed to write blocks");
		tokio::fs::write(
			path.join(LEVEL_DATA_PATH),
			encoder.finish().expect("failed to encode blocks"),
		)
		.await
	}

	/// loads the level
	pub async fn load<P>(path: P) -> std::io::Result<Self>
	where
		P: AsRef<Path>,
	{
		let path = path.as_ref();
		let mut info: Self =
			serde_json::from_str(&tokio::fs::read_to_string(path.join(LEVEL_INFO_PATH)).await?)
				.expect("failed to deserialize level info");
		let blocks_data = tokio::fs::read(path.join(LEVEL_DATA_PATH)).await?;
		let mut decoder = flate2::read::GzDecoder::new(blocks_data.as_slice());
		decoder.read_to_end(&mut info.blocks)?;
		let len = info.x_size * info.y_size * info.z_size;
		if info.blocks.len() != len {
			panic!(
				"level data is not correct size! expected {len}, got {}",
				info.blocks.len()
			);
		}
		Ok(info)
	}
}

/// struct describing a block update for the level to handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockUpdate {
	/// the index of the block to be updated
	pub index: usize,
	/// the block type to set the block to
	pub block: u8,
}

/// weather types for a level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, strum::EnumString, strum::IntoStaticStr)]
#[strum(ascii_case_insensitive)]
pub enum WeatherType {
	Sunny,
	Raining,
	Snowing,
}

impl Default for WeatherType {
	fn default() -> Self {
		Self::Sunny
	}
}

impl From<&WeatherType> for u8 {
	fn from(value: &WeatherType) -> Self {
		match value {
			WeatherType::Sunny => 0,
			WeatherType::Raining => 1,
			WeatherType::Snowing => 2,
		}
	}
}

impl From<u8> for WeatherType {
	fn from(value: u8) -> Self {
		match value {
			1 => Self::Raining,
			2 => Self::Snowing,
			_ => Self::Sunny,
		}
	}
}
