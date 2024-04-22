use std::collections::BTreeSet;

use crate::{packet::server::ServerPacket, util::neighbors};

use self::block::BLOCK_INFO;

pub mod block;
pub mod generation;

/// a classic level
#[derive(Debug, Clone)]
pub struct Level {
	/// the size of the level in the X direction
	pub x_size: usize,
	/// the size of the level in the Y direction
	pub y_size: usize,
	/// the size of the level in the Z direction
	pub z_size: usize,

	/// the blocks which make up the level
	pub blocks: Vec<u8>,
	/// index of blocks which need to be updated in the next tick
	pub awaiting_update: BTreeSet<usize>,
	/// list of updates to apply to the world on the next tick
	pub updates: Vec<BlockUpdate>,
}

impl Level {
	/// creates a new level with the given dimensions
	pub fn new(x_size: usize, y_size: usize, z_size: usize) -> Self {
		Self {
			x_size,
			y_size,
			z_size,
			blocks: vec![0; x_size * y_size * z_size],
			awaiting_update: Default::default(),
			updates: Default::default(),
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
}

/// struct describing a block update for the level to handle
#[derive(Debug, Clone)]
pub struct BlockUpdate {
	/// the index of the block to be updated
	pub index: usize,
	/// the block type to set the block to
	pub block: u8,
}
