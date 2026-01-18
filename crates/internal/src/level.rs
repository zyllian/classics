use std::{
	any::TypeId,
	collections::{BTreeMap, BTreeSet},
	io::{Read, Write},
	path::Path,
};

use bevy_reflect::{PartialReflect, Struct};
use serde::{Deserialize, Serialize};

use crate::{
	error::GeneralError, packet::server::ServerPacket, player::SavablePlayerData,
	util::neighbors_full,
};

use self::block::BLOCK_INFO;

pub mod block;

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
	/// the level's level rules
	#[serde(default)]
	pub rules: LevelRules,

	/// index of blocks which need to be updated in the next tick
	#[serde(default)]
	pub awaiting_update: BTreeSet<usize>,
	/// index of blocks which are eligible for random tick updates
	#[serde(default)]
	pub possible_random_updates: Vec<usize>,
	/// list of updates to apply to the world on the next tick
	#[serde(skip)]
	pub updates: Vec<BlockUpdate>,
	#[serde(skip)]
	pub save_now: bool,

	#[serde(default)]
	pub player_data: BTreeMap<String, SavablePlayerData>,
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
			rules: Default::default(),
			awaiting_update: Default::default(),
			possible_random_updates: Default::default(),
			updates: Default::default(),
			save_now: false,
			player_data: Default::default(),
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
			for (nx, ny, nz) in neighbors_full(self, x, y, z) {
				let info = BLOCK_INFO
					.get(&self.get_block(nx, ny, nz))
					.expect("missing block");
				if info.needs_update_when_neighbor_changed {
					self.awaiting_update.insert(self.index(nx, ny, nz));
				}
			}
		}

		packets
	}

	/// updates player data for the level
	pub fn update_player_data(&mut self, player_data: Vec<(String, SavablePlayerData)>) {
		for (username, data) in player_data {
			self.player_data.insert(username, data);
		}
	}

	/// saves the level
	pub async fn save<P>(&self, path: P) -> Result<(), GeneralError>
	where
		P: AsRef<Path>,
	{
		let path = path.as_ref();
		tokio::fs::create_dir_all(path).await?;
		tokio::fs::write(
			path.join(LEVEL_INFO_PATH),
			serde_json::to_string_pretty(self)?,
		)
		.await?;
		let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
		encoder.write_all(&self.blocks)?;
		Ok(tokio::fs::write(path.join(LEVEL_DATA_PATH), encoder.finish()?).await?)
	}

	/// loads the level
	pub async fn load<P>(path: P) -> Result<Self, GeneralError>
	where
		P: AsRef<Path>,
	{
		let path = path.as_ref();
		let mut info: Self =
			serde_json::from_str(&tokio::fs::read_to_string(path.join(LEVEL_INFO_PATH)).await?)?;
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

		// queue updates for blocks which didn't update properly before (i.e. for flowing water if fluid_spreads was set to false)
		for (i, id) in info.blocks.iter().enumerate() {
			if let Some(block) = BLOCK_INFO.get(id) {
				if block.block_type.needs_update_on_place() {
					info.awaiting_update.insert(i);
				}
			}
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

/// Struct for rules in the level.
#[derive(Debug, Clone, Serialize, Deserialize, bevy_reflect::Reflect)]
pub struct LevelRules {
	/// whether fluids should spread in the level
	#[serde(default = "level_rules::fluid_spread")]
	pub fluid_spread: bool,
	/// the number of blocks which should receive random tick updates
	#[serde(default = "level_rules::random_tick_updates")]
	pub random_tick_updates: u64,
	/// the chance that grass will spread to an adjacent dirt block when randomly updated
	#[serde(default = "level_rules::grass_spread_chance")]
	pub grass_spread_chance: u64,
}

impl LevelRules {
	/// Gets information about all level rules.
	pub fn get_all_rules_info(&self) -> Option<BTreeMap<String, String>> {
		let info = self.get_represented_struct_info()?;
		let mut rules = BTreeMap::new();
		for name in info.field_names() {
			rules.insert(name.to_string(), self.get_rule(name)?);
		}
		Some(rules)
	}

	/// Gets information about a single level rule.
	pub fn get_rule(&self, name: &str) -> Option<String> {
		let info = self.get_represented_struct_info()?;
		Some(format!(
			"{:?} ({})",
			self.field(name)?,
			info.field(name)?.type_path_table().ident()?
		))
	}

	/// Sets a rule to the given value if possible.
	pub fn set_rule(&mut self, name: &str, value: &str) -> Result<(), String> {
		let bool_type_id = TypeId::of::<bool>();
		let f64_type_id = TypeId::of::<f64>();
		let u64_type_id = TypeId::of::<u64>();
		let string_type_id = TypeId::of::<String>();

		fn parse_and_apply<T>(value: &str, field_mut: &mut dyn PartialReflect) -> Result<(), String>
		where
			T: std::str::FromStr + PartialReflect,
		{
			let value = value
				.parse::<T>()
				.map_err(|_| "Failed to parse value".to_string())?;
			field_mut.apply(value.as_partial_reflect());
			Ok(())
		}

		let info = self
			.get_represented_struct_info()
			.ok_or_else(|| "Failed to get field info".to_string())?;
		let field = info
			.field(name)
			.ok_or_else(|| format!("Unknown field: {name}"))?;
		let field_mut = self
			.field_mut(name)
			.ok_or_else(|| format!("Unknown field: {name}"))?;
		let id = field.type_id();
		if id == bool_type_id {
			parse_and_apply::<bool>(value, field_mut)?;
		} else if id == f64_type_id {
			parse_and_apply::<f64>(value, field_mut)?;
		} else if id == u64_type_id {
			parse_and_apply::<u64>(value, field_mut)?;
		} else if id == string_type_id {
			parse_and_apply::<String>(value, field_mut)?;
		} else {
			return Err(format!("Field has unknown type: {}", field.type_path()));
		};

		Ok(())
	}
}

mod level_rules {
	pub fn fluid_spread() -> bool {
		true
	}

	pub fn random_tick_updates() -> u64 {
		1000
	}

	pub fn grass_spread_chance() -> u64 {
		2048
	}
}

impl Default for LevelRules {
	fn default() -> Self {
		use level_rules::*;
		Self {
			fluid_spread: fluid_spread(),
			random_tick_updates: random_tick_updates(),
			grass_spread_chance: grass_spread_chance(),
		}
	}
}
