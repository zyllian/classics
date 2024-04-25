use half::f16;
use safer_bytes::{error::Truncated, SafeBuf};

pub mod client;
pub mod client_extended;
pub mod server;

/// length of classic strings
pub const STRING_LENGTH: usize = 64;
/// length of classic level chunk arrays
pub const ARRAY_LENGTH: usize = 1024;
/// units in an f16 unit
pub const F16_UNITS: f32 = 32.0;
/// the magic number to check whether the client supports extensions
pub const EXTENSION_MAGIC_NUMBER: u8 = 0x42;

/// information about a packet extension
#[derive(Debug, PartialEq, Eq)]
pub struct ExtInfo {
	/// the extension's name
	pub ext_name: String,
	/// the extension's version
	pub version: i32,
	/// the bitmask for the extension
	pub bitmask: ExtBitmask,
}

impl ExtInfo {
	/// creates new extension info
	pub const fn new(ext_name: String, version: i32, bitmask: ExtBitmask) -> Self {
		Self {
			ext_name,
			version,
			bitmask,
		}
	}
}

/// trait extending the `SafeBuf` type
pub trait SafeBufExtension: SafeBuf {
	/// tries to get the next f16 in the buffer
	fn try_get_f16(&mut self) -> Result<f16, Truncated>;
	/// tries to get the next string in the buffer
	fn try_get_string(&mut self) -> Result<String, Truncated>;
}

impl<T> SafeBufExtension for T
where
	T: SafeBuf,
{
	fn try_get_f16(&mut self) -> Result<f16, Truncated> {
		self.try_get_i16()
			.map(|v| f16::from_f32(v as f32 / F16_UNITS))
	}

	fn try_get_string(&mut self) -> Result<String, Truncated> {
		let mut chars: Vec<char> = Vec::new();
		for _ in 0..STRING_LENGTH {
			chars.push(self.try_get_u8()? as char);
		}
		Ok(String::from_iter(chars).trim().to_string())
	}
}

/// helper for writing a packet
#[derive(Debug, Default)]
pub struct PacketWriter {
	raw_packet: Vec<u8>,
}

impl PacketWriter {
	/// gets the actual raw packet data from the writer
	pub fn into_raw_packet(self) -> Vec<u8> {
		self.raw_packet
	}

	/// writes a u8 to the packet
	pub fn write_u8(mut self, b: u8) -> Self {
		self.raw_packet.push(b);
		self
	}

	/// writes an i8 to the packet
	fn write_i8(self, b: i8) -> Self {
		self.write_u8(b as u8)
	}

	/// writes a bool to the packet
	fn write_bool(self, b: bool) -> Self {
		self.write_u8(if b { 1 } else { 0 })
	}

	/// writes a u16 to the packet
	fn write_u16(self, sh: u16) -> Self {
		let mut s = self;
		for b in sh.to_be_bytes() {
			s = s.write_u8(b);
		}
		s
	}

	/// writes an i16 to the packet
	fn write_i16(self, sh: i16) -> Self {
		self.write_u16(sh as u16)
	}

	/// writes an f16 to the packet
	fn write_f16(self, f: f16) -> Self {
		let r = (f.to_f32() * F16_UNITS) as i16;
		self.write_i16(r)
	}

	/// writes an i32 to the packet
	fn write_i32(self, i: i32) -> Self {
		let mut s = self;
		for b in i.to_be_bytes() {
			s = s.write_u8(b);
		}
		s
	}

	/// writes a string to the packet
	fn write_string(self, str: &str) -> Self {
		let mut s = self;
		for b in str
			.as_bytes()
			.iter()
			.copied()
			.chain(Some(0x20).into_iter().cycle())
			.take(STRING_LENGTH)
		{
			s = s.write_u8(b);
		}
		s
	}

	/// writes an array of the given length to the packet
	fn write_array_of_length(self, bytes: &[u8], len: usize) -> Self {
		let mut s = self;
		for i in 0..len {
			s = s.write_u8(bytes.get(i).copied().unwrap_or_default());
		}
		s
	}

	/// writes an array of default length to the packet
	fn write_array(self, bytes: &[u8]) -> Self {
		self.write_array_of_length(bytes, ARRAY_LENGTH)
	}
}

/// bitmask for enabled extensions
/// values should not be saved to disk or sent over network! no guarantees on them remaining the same between versions
#[bitmask_enum::bitmask(u64)]
pub enum ExtBitmask {
	ClickDistance,
	CustomBlocks,
	HeldBlock,
	EmoteFix,
	TextHotKey,
	ExtPlayerList,
	EnvColors,
	SelectionCuboid,
	BlockPermissions,
	ChangeModel,
	EnvMapAppearance,
	EnvWeatherType,
	HackControl,
	MessageTypes,
	PlayerClick,
	LongerMessages,
	FullCP437,
	BlockDefinitions,
	BlockDefinitionsExt,
	BulkBlockUpdate,
	TextColors,
	EnvMapAspect,
	EntityProperty,
	ExtEntityPositions,
	TwoWayPing,
	InventoryOrder,
	InstantMOTD,
	ExtendedBlocks,
	FastMap,
	ExtendedTextures,
	SetHotbar,
	SetSpawnpoint,
	VelocityControl,
	CustomParticles,
	CustomModels_v2,
	ExtEntityTeleport,
}

impl ExtBitmask {
	/// gets info about a specific extension
	fn info(self) -> Option<ExtInfo> {
		// TODO: add entries as extensions are supported
		Some(match self {
			Self::CustomBlocks => ExtInfo::new("CustomBlocks".to_string(), 1, Self::CustomBlocks),
			// this isn't actually used by the server at all, but it technically sort of implements it
			Self::HeldBlock => ExtInfo::new("HeldBlock".to_string(), 1, Self::HeldBlock),
			Self::EmoteFix => ExtInfo::new("EmoteFix".to_string(), 1, Self::EmoteFix),
			// TODO: render CP437 properly in server output
			Self::FullCP437 => ExtInfo::new("FullCP437".to_string(), 1, Self::FullCP437),
			Self::EnvWeatherType => {
				ExtInfo::new("EnvWeatherType".to_string(), 1, Self::EnvWeatherType)
			}
			_ => return None,
		})
	}

	/// gets info about all extensions
	pub fn all_contained_info(self) -> Vec<ExtInfo> {
		[
			Self::ClickDistance,
			Self::CustomBlocks,
			Self::HeldBlock,
			Self::EmoteFix,
			Self::TextHotKey,
			Self::ExtPlayerList,
			Self::EnvColors,
			Self::SelectionCuboid,
			Self::BlockPermissions,
			Self::ChangeModel,
			Self::EnvMapAppearance,
			Self::EnvWeatherType,
			Self::HackControl,
			Self::MessageTypes,
			Self::PlayerClick,
			Self::LongerMessages,
			Self::FullCP437,
			Self::BlockDefinitions,
			Self::BlockDefinitionsExt,
			Self::BulkBlockUpdate,
			Self::TextColors,
			Self::EnvMapAspect,
			Self::EntityProperty,
			Self::ExtEntityPositions,
			Self::TwoWayPing,
			Self::InventoryOrder,
			Self::InstantMOTD,
			Self::ExtendedBlocks,
			Self::FastMap,
			Self::ExtendedTextures,
			Self::SetHotbar,
			Self::SetSpawnpoint,
			Self::VelocityControl,
			Self::CustomParticles,
			Self::CustomModels_v2,
			Self::ExtEntityTeleport,
		]
		.into_iter()
		.filter_map(|flag| (self & flag).info())
		.collect()
	}
}
