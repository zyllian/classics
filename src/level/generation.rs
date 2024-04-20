use internment::Intern;
use rand::Rng;
use serde::{Deserialize, Serialize};

use super::{block::BLOCK_STRING_ID_MAP, Level};

/// enum for different kinds of level generation
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LevelGeneration {
	/// an empty level
	Empty,
	/// a level where every block up to the given height is randomized
	FullRandom { height: usize },
	/// a flat level with the given preset
	Flat(FlatPreset),
}

/// enum for level presents
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "flat_type")]
pub enum FlatPreset {
	/// the level is mostly stone, then dirt, then a layer of grass on the top
	StoneAndGrass,
	/// the level layers are custom as defined in server config
	Custom { layers: Vec<FlatLayer> },
}

/// description of a flat world's layer
#[derive(Debug, Serialize, Deserialize)]
pub struct FlatLayer {
	/// the block for the layer
	pub block: String,
	/// the depth of the layer
	pub depth: usize,
}

impl LevelGeneration {
	/// generates the level
	pub fn generate<R>(&self, level: &mut Level, rng: &mut R)
	where
		R: Rng,
	{
		match self {
			Self::Empty => {}
			Self::FullRandom { height } => {
				let height = *height.min(&level.y_size);
				for x in 0..level.x_size {
					for y in 0..height {
						for z in 0..level.z_size {
							level.set_block(x, y, z, rng.gen_range(0..49));
						}
					}
				}
			}
			Self::Flat(preset) => {
				let custom_layers;
				let layers_ref;

				match preset {
					FlatPreset::StoneAndGrass => {
						custom_layers = vec![
							FlatLayer {
								block: "stone".to_owned(),
								depth: level.y_size / 2 - 4,
							},
							FlatLayer {
								block: "dirt".to_owned(),
								depth: 3,
							},
							FlatLayer {
								block: "grass".to_owned(),
								depth: 1,
							},
						];
						layers_ref = &custom_layers;
					}
					FlatPreset::Custom { layers } => {
						layers_ref = layers;
					}
				}

				let mut y = 0;
				for layer in layers_ref {
					let block = Intern::new(layer.block.to_string());
					for _ in 0..layer.depth {
						for x in 0..level.x_size {
							for z in 0..level.z_size {
								level.set_block(
									x,
									y,
									z,
									*BLOCK_STRING_ID_MAP
										.get(&block)
										.expect("missing block type!"),
								);
							}
						}
						y += 1;
						if y >= level.y_size {
							return;
						}
					}
				}
			}
		}
	}
}
