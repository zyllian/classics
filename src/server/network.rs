mod extensions;

use std::{io::Write, net::SocketAddr, sync::Arc};

use bytes::BytesMut;
use flate2::{write::GzEncoder, Compression};
use half::f16;
use tokio::{
	io::{AsyncReadExt, AsyncWriteExt},
	net::TcpStream,
	sync::RwLock,
};

use crate::{
	command::Command,
	level::{block::BLOCK_INFO, BlockUpdate, Level},
	packet::{
		client::ClientPacket, server::ServerPacket, ExtBitmask, PacketWriter, ARRAY_LENGTH,
		EXTENSION_MAGIC_NUMBER,
	},
	player::{Player, PlayerType},
	server::config::ServerProtectionMode,
};

use super::ServerData;

async fn next_packet(stream: &mut TcpStream) -> std::io::Result<Option<ClientPacket>> {
	let id = stream.read_u8().await?;

	if let Some(size) = ClientPacket::get_size_from_id(id) {
		let mut buf = BytesMut::zeroed(size);
		stream.read_exact(&mut buf).await?;
		Ok(ClientPacket::read(id, &mut buf))
	} else {
		println!("unknown packet id: {id:0x}");
		Ok(None)
	}
}

async fn write_packets<I>(stream: &mut TcpStream, packets: I) -> std::io::Result<()>
where
	I: Iterator<Item = ServerPacket>,
{
	for packet in packets {
		let writer = PacketWriter::default().write_u8(packet.get_id());
		let msg = packet.write(writer).into_raw_packet();
		stream.write_all(&msg).await?;
	}
	Ok(())
}

pub(super) async fn handle_stream(
	mut stream: TcpStream,
	addr: SocketAddr,
	data: Arc<RwLock<ServerData>>,
) {
	let mut own_id: i8 = -1;
	let r = handle_stream_inner(&mut stream, addr, data.clone(), &mut own_id).await;

	println!("{addr} is no longer connected");
	match r {
		Ok(disconnect_reason) => {
			if let Some(disconnect_reason) = disconnect_reason {
				let packet = ServerPacket::DisconnectPlayer { disconnect_reason };
				let writer = PacketWriter::default().write_u8(packet.get_id());
				let msg = packet.write(writer).into_raw_packet();
				if let Err(e) = stream.write_all(&msg).await {
					eprintln!("Failed to write disconnect packet for <{addr}>: {e}");
				}
			}
		}
		Err(e) => {
			// unexpected eof is expected when clients disconnect
			if e.kind() != std::io::ErrorKind::UnexpectedEof {
				eprintln!("Error in stream handler for <{addr}>: {e}")
			}
		}
	}

	if let Err(e) = stream.shutdown().await {
		eprintln!("Failed to properly shut down stream for <{addr}>: {e}");
	}

	let mut data = data.write().await;
	if let Some(index) = data.players.iter().position(|p| p.id == own_id) {
		let player = data.players.remove(index);
		data.free_player_ids.push(player.id);

		let despawn_packet = ServerPacket::DespawnPlayer { player_id: own_id };
		let message_packet = ServerPacket::Message {
			player_id: own_id,
			message: format!("&e{} has left the server.", player.username),
		};
		for player in &mut data.players {
			player.packets_to_send.push(despawn_packet.clone());
			player.packets_to_send.push(message_packet.clone());
		}
	}
}

async fn handle_stream_inner(
	stream: &mut TcpStream,
	addr: SocketAddr,
	data: Arc<RwLock<ServerData>>,
	own_id: &mut i8,
) -> std::io::Result<Option<String>> {
	let mut reply_queue: Vec<ServerPacket> = Vec::new();

	macro_rules! msg {
		($message:expr) => {
			reply_queue.push(ServerPacket::Message {
				player_id: -1,
				message: $message,
			});
		};
	}

	loop {
		if let Some(player) = data.read().await.players.iter().find(|p| p.id == *own_id) {
			if let Some(msg) = &player.should_be_kicked {
				return Ok(Some(format!("Kicked: {msg}")));
			}
		}

		if let Some(packet) = next_packet(stream).await? {
			match packet {
				ClientPacket::PlayerIdentification {
					protocol_version,
					username,
					verification_key,
					magic_number,
				} => {
					if protocol_version != 0x07 {
						return Ok(Some("Unknown protocol version! Please connect with a classic 0.30-compatible client.".to_string()));
					}

					let zero = f16::from_f32(0.0);

					let mut data = data.write().await;

					match &data.config.protection_mode {
						ServerProtectionMode::None => {}
						ServerProtectionMode::Password(password) => {
							if verification_key != *password {
								return Ok(Some("Incorrect password!".to_string()));
							}
						}
						ServerProtectionMode::PasswordsByUser(passwords) => {
							if !passwords
								.get(&username)
								.map(|password| verification_key == *password)
								.unwrap_or_default()
							{
								return Ok(Some("Incorrect password!".to_string()));
							}
						}
					}

					for player in &data.players {
						if player.username == username {
							return Ok(Some("Player with username already connected!".to_string()));
						}
					}

					*own_id = data
						.free_player_ids
						.pop()
						.unwrap_or_else(|| data.players.len() as i8);

					let player_type = data
						.config
						.player_perms
						.get(&username)
						.copied()
						.unwrap_or_default();

					let mut player = Player {
						_addr: addr,
						id: *own_id, // TODO: actually assign user ids
						username,
						x: zero,
						y: zero,
						z: zero,
						yaw: 0,
						pitch: 0,
						permissions: player_type,
						extensions: ExtBitmask::none(),
						packets_to_send: Vec::new(),
						should_be_kicked: None,
					};

					if magic_number == EXTENSION_MAGIC_NUMBER {
						player.extensions = extensions::get_supported_extensions(stream).await?;
					}

					reply_queue.push(ServerPacket::ServerIdentification {
						protocol_version: 0x07,
						server_name: data.config.name.clone(),
						server_motd: data.config.motd.clone(),
						user_type: player_type,
					});

					println!("generating level packets");
					reply_queue.extend(build_level_packets(&data.level).into_iter());

					if player.extensions.contains(ExtBitmask::EnvWeatherType) {
						reply_queue.push(ServerPacket::EnvWeatherType {
							weather_type: data.level.weather,
						});
					}

					let username = player.username.clone();

					let (spawn_x, spawn_y, spawn_z, spawn_yaw, spawn_pitch) =
						if let Some(spawn) = &data.config.spawn {
							(spawn.x, spawn.y, spawn.z, spawn.yaw, spawn.pitch)
						} else {
							(16.5, (data.level.y_size / 2 + 2) as f32, 16.5, 0, 0)
						};

					let (spawn_x, spawn_y, spawn_z) = (
						f16::from_f32(spawn_x),
						f16::from_f32(spawn_y),
						f16::from_f32(spawn_z),
					);

					player.x = spawn_x;
					player.y = spawn_y;
					player.z = spawn_z;
					player.yaw = spawn_yaw;
					player.pitch = spawn_pitch;
					data.players.push(player);

					let spawn_packet = ServerPacket::SpawnPlayer {
						player_id: *own_id,
						player_name: username.clone(),
						x: spawn_x,
						y: spawn_y,
						z: spawn_z,
						yaw: spawn_yaw,
						pitch: spawn_pitch,
					};
					let message_packet = ServerPacket::Message {
						player_id: *own_id,
						message: format!("&e{} has joined the server.", username),
					};
					for player in &mut data.players {
						player.packets_to_send.push(spawn_packet.clone());
						if player.id != *own_id {
							reply_queue.push(ServerPacket::SpawnPlayer {
								player_id: player.id,
								player_name: player.username.clone(),
								x: player.x,
								y: player.y,
								z: player.z,
								yaw: player.yaw,
								pitch: player.pitch,
							});
							player.packets_to_send.push(message_packet.clone());
						}
					}
					msg!("&dWelcome to the server! Enjoyyyyyy".to_string());
					reply_queue.push(ServerPacket::UpdateUserType {
						user_type: PlayerType::Operator,
					});
				}
				ClientPacket::SetBlock {
					x,
					y,
					z,
					mode,
					block_type,
				} => {
					let block_type = if mode == 0x00 { 0 } else { block_type };
					let mut data = data.write().await;

					// kick players if they attempt to place a block out of bounds
					if x.clamp(0, data.level.x_size as i16 - 1) != x
						|| y.clamp(0, data.level.y_size as i16 - 1) != y
						|| z.clamp(0, data.level.z_size as i16 - 1) != z
					{
						return Ok(Some("Attempt to place block out of bounds".to_string()));
					}

					let new_block_info = BLOCK_INFO.get(&block_type);
					if new_block_info.is_none() {
						msg!(format!("&cUnknown block ID: 0x{:0x}", block_type));
						continue;
					}
					let new_block_info = new_block_info.expect("will never fail");
					let mut cancel = false;
					let block = data.level.get_block(x as usize, y as usize, z as usize);
					let block_info = BLOCK_INFO
						.get(&block)
						.expect("missing block information for block!");

					// check if player has ability to place/break these blocks
					let player_type = data
						.players
						.iter()
						.find_map(|p| (p.id == *own_id).then_some(p.permissions))
						.unwrap_or_default();
					if player_type < new_block_info.place_permissions {
						cancel = true;
						msg!("&cNot allow to place this block.".to_string());
					} else if player_type < block_info.break_permissions {
						cancel = true;
						msg!("&cNot allowed to break this block.".to_string());
					}

					if cancel {
						reply_queue.push(ServerPacket::SetBlock {
							x,
							y,
							z,
							block_type: block,
						});
						continue;
					}
					let (x, y, z) = (x as usize, y as usize, z as usize);
					let index = data.level.index(x, y, z);
					data.level.updates.push(BlockUpdate {
						index,
						block: block_type,
					});
					if new_block_info.block_type.needs_update_on_place() {
						data.level.awaiting_update.insert(index);
					}
				}
				ClientPacket::PositionOrientation {
					_player_id_or_held_block: _,
					x,
					y,
					z,
					yaw,
					pitch,
				} => {
					let mut data = data.write().await;

					let player = data
						.players
						.iter_mut()
						.find(|p| p.id == *own_id)
						.expect("missing player");
					player.x = x;
					player.y = y;
					player.z = z;
					player.yaw = yaw;
					player.pitch = pitch;

					data.spread_packet(ServerPacket::SetPositionOrientation {
						player_id: *own_id,
						x,
						y,
						z,
						yaw,
						pitch,
					});
				}
				ClientPacket::Message { player_id, message } => {
					let mut data = data.write().await;

					if let Some(message) = message.strip_prefix(Command::PREFIX) {
						match Command::parse(message) {
							Ok(cmd) => {
								for message in cmd.process(&mut data, *own_id) {
									msg!(message);
								}
							}
							Err(msg) => {
								msg!(format!("&c{msg}"));
							}
						}
					} else {
						println!("{message}");
						let message = format!(
							"&f<{}> {message}",
							data.players
								.iter()
								.find(|p| p.id == *own_id)
								.expect("should never fail")
								.username
						);
						data.spread_packet(ServerPacket::Message { player_id, message });
					}
				}

				ClientPacket::Extended(_packet) => {
					// extended packets!
					return Ok(Some(
						"Unexpected extension packet in this phase!".to_string(),
					));
					// match packet {
					// 	packet => {
					// 		println!("improper client packet for this phase!: {packet:#?}");
					// 		return Ok(Some(
					// 			"Client sent invalid packet for this phase".to_string(),
					// 		));
					// 	}
					// }
				}
			}
		}

		let mut data = data.write().await;
		if let Some(player) = data.players.iter_mut().find(|p| p.id == *own_id) {
			for mut packet in player.packets_to_send.drain(..) {
				if let Some(id) = packet.get_player_id() {
					if id == *own_id {
						if !packet.should_echo() {
							continue;
						}
						packet.set_player_id(-1);
					}
				}
				reply_queue.push(packet);
			}
		}

		write_packets(stream, reply_queue.drain(..)).await?;
	}
}

/// helper to put together packets that need to be sent to send full level data for the given level
fn build_level_packets(level: &Level) -> Vec<ServerPacket> {
	let mut packets: Vec<ServerPacket> = vec![ServerPacket::LevelInitialize {}];

	// TODO: the type conversions in here may be weird idk
	let volume = level.x_size * level.y_size * level.z_size;
	let mut data = Vec::with_capacity(volume + 4);
	data.extend_from_slice(&(volume as i32).to_be_bytes());
	data.extend_from_slice(&level.blocks);

	let mut e = GzEncoder::new(Vec::new(), Compression::best());
	e.write_all(&data).expect("failed to gzip level data");
	let data = e.finish().expect("failed to gzip level data");
	let data_len = data.len();
	let mut total_bytes = 0;

	for chunk in data.chunks(ARRAY_LENGTH) {
		let chunk_len = chunk.len();
		let percent_complete = (total_bytes * 100 / data_len) as u8;
		packets.push(ServerPacket::LevelDataChunk {
			chunk_length: chunk_len as i16,
			chunk_data: chunk.to_vec(),
			percent_complete,
		});

		total_bytes += chunk_len;
	}

	packets.push(ServerPacket::LevelFinalize {
		x_size: level.x_size as i16,
		y_size: level.y_size as i16,
		z_size: level.z_size as i16,
	});

	packets
}
