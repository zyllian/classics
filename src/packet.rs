use half::f16;

pub mod client;
pub mod server;

/// length of classic strings
pub const STRING_LENGTH: usize = 64;
/// length of classic level chunk arrays
pub const ARRAY_LENGTH: usize = 1024;
/// units in an f16 unit
pub const F16_UNITS: f32 = 32.0;

/// helper for reading packets
#[derive(Debug)]
pub struct PacketReader<'p> {
	raw_packet: &'p [u8],
	cursor: usize,
}

impl<'p> PacketReader<'p> {
	/// creates a new packet reader from the given packet data
	pub fn new(raw_packet: &'p [u8]) -> Self {
		Self {
			raw_packet,
			cursor: 0,
		}
	}

	/// gets the next u8 in the packet, if any
	fn next_u8(&mut self) -> Option<u8> {
		let r = self.raw_packet.get(self.cursor).copied();
		self.cursor = self.cursor.checked_add(1).unwrap_or(self.cursor);
		r
	}

	/// gets the next i8 in the packet, if any
	fn next_i8(&mut self) -> Option<i8> {
		self.next_u8().map(|b| b as i8)
	}

	/// gets the next u16 in the packet, if any
	fn next_u16(&mut self) -> Option<u16> {
		Some(u16::from_be_bytes([self.next_u8()?, self.next_u8()?]))
	}

	/// gets the next i16 in the packet, if any
	fn next_i16(&mut self) -> Option<i16> {
		self.next_u16().map(|s| s as i16)
	}

	/// gets the next f16 in the packet, if any
	fn next_f16(&mut self) -> Option<f16> {
		self.next_i16().map(|v| f16::from_f32(v as f32 / F16_UNITS))
	}

	/// gets the next string in the packet, if any
	fn next_string(&mut self) -> Option<String> {
		let mut chars: Vec<char> = Vec::new();
		for _ in 0..STRING_LENGTH {
			chars.push(self.next_u8()? as char);
		}
		Some(String::from_iter(chars).trim().to_string())
	}

	/// gets the next array of the given length in the packet, if any
	fn next_array_of_length(&mut self, len: usize) -> Option<Vec<u8>> {
		let mut bytes: Vec<u8> = Vec::new();
		let mut append = true;
		for _ in 0..len {
			let b = self.next_u8()?;
			if append {
				if b == 0 {
					append = false;
				} else {
					bytes.push(b);
				}
			}
		}
		Some(bytes)
	}

	/// gets the next array of default size in the packet, if any
	fn next_array(&mut self) -> Option<Vec<u8>> {
		self.next_array_of_length(ARRAY_LENGTH)
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
