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
}

impl Level {
	/// creates a new level with the given dimensions
	pub fn new(x_size: usize, y_size: usize, z_size: usize) -> Self {
		Self {
			x_size,
			y_size,
			z_size,
			blocks: vec![0; x_size * y_size * z_size],
		}
	}

	/// gets the index for a given block position
	pub fn index(&self, x: usize, y: usize, z: usize) -> usize {
		x + z * self.x_size + y * self.x_size * self.z_size
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
}
