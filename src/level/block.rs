use std::{collections::BTreeMap, sync::LazyLock};

use internment::Intern;

use crate::player::PlayerType;

/// information about all blocks implemented
pub static BLOCK_INFO: LazyLock<BTreeMap<u8, BlockInfo>> = LazyLock::new(|| {
	[
		(0x00, BlockInfo::new("air").block_type(BlockType::NonSolid)),
		(0x01, BlockInfo::new("stone")),
		(0x02, BlockInfo::new("grass")),
		(0x03, BlockInfo::new("dirt")),
		(0x04, BlockInfo::new("cobblestone")),
		(0x05, BlockInfo::new("planks")),
		(
			0x06,
			BlockInfo::new("sapling").block_type(BlockType::NonSolid),
		),
		(
			0x07,
			BlockInfo::new("bedrock").perm(PlayerType::Operator, PlayerType::Operator),
		),
		(
			0x08,
			BlockInfo::new("water_flowing")
				.block_type(BlockType::FluidFlowing {
					stationary: 0x09,
					ticks_to_spread: 3,
				})
				.perm(PlayerType::Operator, PlayerType::Normal),
		),
		(
			0x09,
			BlockInfo::new("water_stationary")
				.block_type(BlockType::FluidStationary { moving: 0x08 })
				.perm(PlayerType::Operator, PlayerType::Normal),
		),
		(
			0x0a,
			BlockInfo::new("lava_flowing")
				.block_type(BlockType::FluidFlowing {
					stationary: 0x0b,
					ticks_to_spread: 15,
				})
				.perm(PlayerType::Operator, PlayerType::Normal),
		),
		(
			0x0b,
			BlockInfo::new("lava_stationary")
				.block_type(BlockType::FluidStationary { moving: 0x0a })
				.perm(PlayerType::Operator, PlayerType::Normal),
		),
		(0x0c, BlockInfo::new("sand")),
		(0x0d, BlockInfo::new("gravel")),
		(0x0e, BlockInfo::new("gold_ore")),
		(0x0f, BlockInfo::new("iron_ore")),
		(0x10, BlockInfo::new("coal_ore")),
		(0x11, BlockInfo::new("wood")),
		(0x12, BlockInfo::new("leaves")),
		(0x13, BlockInfo::new("sponge")),
		(0x14, BlockInfo::new("glass")),
		(0x15, BlockInfo::new("cloth_red")),
		(0x16, BlockInfo::new("cloth_orange")),
		(0x17, BlockInfo::new("cloth_yellow")),
		(0x18, BlockInfo::new("cloth_chartreuse")),
		(0x19, BlockInfo::new("cloth_green")),
		(0x1a, BlockInfo::new("cloth_spring_green")),
		(0x1b, BlockInfo::new("cloth_cyan")),
		(0x1c, BlockInfo::new("cloth_capri")),
		(0x1d, BlockInfo::new("cloth_ultramarine")),
		(0x1e, BlockInfo::new("cloth_violet")),
		(0x1f, BlockInfo::new("cloth_purple")),
		(0x20, BlockInfo::new("cloth_magenta")),
		(0x21, BlockInfo::new("cloth_rose")),
		(0x22, BlockInfo::new("cloth_dark_gray")),
		(0x23, BlockInfo::new("cloth_light_gray")),
		(0x24, BlockInfo::new("cloth_white")),
		(
			0x25,
			BlockInfo::new("flower").block_type(BlockType::NonSolid),
		),
		(0x26, BlockInfo::new("rose").block_type(BlockType::NonSolid)),
		(
			0x27,
			BlockInfo::new("brown_mushroom").block_type(BlockType::NonSolid),
		),
		(
			0x28,
			BlockInfo::new("red_mushroom").block_type(BlockType::NonSolid),
		),
		(0x29, BlockInfo::new("gold_block")),
		(0x2a, BlockInfo::new("iron_block")),
		(0x2b, BlockInfo::new("double_slab")),
		(0x2c, BlockInfo::new("slab").block_type(BlockType::Slab)),
		(0x2d, BlockInfo::new("bricks")),
		(0x2e, BlockInfo::new("tnt")),
		(0x2f, BlockInfo::new("bookshelf")),
		(0x30, BlockInfo::new("mossy_cobblestone")),
		(0x31, BlockInfo::new("obsidian")),
	]
	.into()
});

/// map of block string ids to their byte ids
pub static BLOCK_STRING_ID_MAP: LazyLock<BTreeMap<Intern<String>, u8>> = LazyLock::new(|| {
	BLOCK_INFO
		.iter()
		.map(|(id, info)| (info.str_id, *id))
		.collect()
});

/// information about a block type
#[derive(Debug)]
pub struct BlockInfo {
	/// the block's string id
	pub str_id: Intern<String>,
	/// the type of block
	pub block_type: BlockType,
	/// permissions needed to place this block
	pub place_permissions: PlayerType,
	/// permissions needed to break this block (includes replacing fluids)
	pub break_permissions: PlayerType,
}

impl BlockInfo {
	/// creates a new block info
	pub fn new(str_id: &'static str) -> Self {
		Self {
			str_id: Intern::new(str_id.to_owned()),
			block_type: BlockType::Solid,
			place_permissions: PlayerType::Normal,
			break_permissions: PlayerType::Normal,
		}
	}

	/// sets the info's block type
	pub const fn block_type(mut self, block_type: BlockType) -> Self {
		self.block_type = block_type;
		self
	}

	/// sets placement and breaking permissions for the info
	pub const fn perm(mut self, place: PlayerType, brk: PlayerType) -> Self {
		self.place_permissions = place;
		self.break_permissions = brk;
		self
	}
}

/// types of blocks
#[derive(Debug)]
pub enum BlockType {
	/// a regular solid block
	Solid,
	/// a block which has no collision
	NonSolid,
	/// a slab
	Slab,
	/// fluid which is actively flowing
	FluidFlowing {
		stationary: u8,
		ticks_to_spread: usize,
	},
	/// fluid which is stationary
	FluidStationary { moving: u8 },
}

impl BlockType {
	/// gets whether this block type needs an update after being placed
	#[allow(clippy::match_like_matches_macro)]
	pub fn needs_update_on_place(&self) -> bool {
		match self {
			BlockType::FluidFlowing { .. } => true,
			_ => false,
		}
	}

	/// gets whether this block type needs an update when one of it's direct neighbors changes
	#[allow(clippy::match_like_matches_macro)]
	pub fn needs_update_when_neighbor_changed(&self) -> bool {
		match self {
			BlockType::FluidStationary { .. } => true,
			_ => false,
		}
	}
}
