use half::f16;
use safer_bytes::{error::Truncated, SafeBuf};

pub mod client;
pub mod server;

/// length of classic strings
pub const STRING_LENGTH: usize = 64;
/// length of classic level chunk arrays
pub const ARRAY_LENGTH: usize = 1024;
/// units in an f16 unit
pub const F16_UNITS: f32 = 32.0;

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
