use std::{collections::VecDeque, io::Write, net::SocketAddr, sync::Arc};

use flate2::{write::GzEncoder, Compression};
use half::f16;
use tokio::{
	io::{AsyncWriteExt, Interest},
	net::TcpStream,
	sync::RwLock,
};

use crate::{
	level::Level,
	packet::{
		client::ClientPacket, server::ServerPacket, PacketReader, PacketWriter, ARRAY_LENGTH,
	},
	player::{Player, PlayerType},
};

use super::ServerData;

pub(super) async fn handle_stream(
	mut stream: TcpStream,
	addr: SocketAddr,
	data: Arc<RwLock<ServerData>>,
) -> std::io::Result<()> {
	let mut own_id: i8 = -1;
	let r = handle_stream_inner(&mut stream, addr, data.clone(), &mut own_id).await;

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
		Err(e) => eprintln!("Error in stream handler for <{addr}>: {e}"),
	}

	if let Err(e) = stream.shutdown().await {
		eprintln!("Failed to properly shut down stream for <{addr}>: {e}");
	}

	let mut data = data.write().await;
	if let Some(index) = data.players.iter().position(|p| p.id == own_id) {
		let player = data.players.remove(index);

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

	Ok(())
}

async fn handle_stream_inner(
	stream: &mut TcpStream,
	addr: SocketAddr,
	data: Arc<RwLock<ServerData>>,
	own_id: &mut i8,
) -> std::io::Result<Option<String>> {
	const BUF_SIZE: usize = 130;

	let mut reply_queue: VecDeque<ServerPacket> = VecDeque::new();
	let mut packet_buf = [0u8];
	let mut read_buf;

	loop {
		let ready = stream
			.ready(Interest::READABLE | Interest::WRITABLE)
			.await?;

		if ready.is_read_closed() {
			println!("disconnecting {addr}");
			break;
		}

		if ready.is_readable() {
			match stream.try_read(&mut packet_buf) {
				Ok(n) => {
					if n == 1 {
						read_buf = [0; BUF_SIZE];
						match stream.try_read(&mut read_buf) {
							Ok(_n) => {}
							Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
							Err(e) => return Err(e),
						}

						let mut reader = PacketReader::new(&read_buf);

						if let Some(packet) = ClientPacket::read(packet_buf[0], &mut reader) {
							match packet {
								ClientPacket::PlayerIdentification {
									protocol_version,
									username,
									verification_key,
									_unused,
								} => {
									if protocol_version != 0x07 {
										return Ok(Some("Unknown protocol version! Please connect with a classic 0.30-compatible client.".to_string()));
									}

									let zero = f16::from_f32(0.0);

									let mut data = data.write().await;

									if let Some(password) = &data.config.password {
										if verification_key != *password {
											return Ok(Some("Incorrect password!".to_string()));
										}
									}

									for player in &data.players {
										if player.username == username {
											return Ok(Some(
												"Player with username already connected!"
													.to_string(),
											));
										}
									}

									*own_id = data
										.free_player_ids
										.pop()
										.unwrap_or_else(|| data.players.len() as i8);

									let player = Player {
										_addr: addr,
										id: *own_id, // TODO: actually assign user ids
										username,
										// TODO: properly assign spawn stuff
										x: zero,
										y: zero,
										z: zero,
										yaw: 0,
										pitch: 0,
										player_type: PlayerType::Normal,
										packets_to_send: Vec::new(),
									};

									reply_queue.push_back(ServerPacket::ServerIdentification {
										protocol_version: 0x07,
										server_name: data.config.name.clone(),
										server_motd: data.config.motd.clone(),
										user_type: PlayerType::Normal,
									});

									println!("generating level packets");
									reply_queue
										.extend(build_level_packets(&data.level).into_iter());

									let username = player.username.clone();
									data.players.push(player);

									let (spawn_x, spawn_y, spawn_z) =
										if let Some(spawn) = &data.config.spawn {
											(spawn.x, spawn.y, spawn.z)
										} else {
											(16, data.level.y_size / 2 + 2, 16)
										};

									let spawn_packet = ServerPacket::SpawnPlayer {
										player_id: *own_id,
										player_name: username.clone(),
										x: f16::from_f32(spawn_x as f32 + 0.5),
										y: f16::from_f32(spawn_y as f32),
										z: f16::from_f32(spawn_z as f32 + 0.5),
										yaw: 0,
										pitch: 0,
									};
									let message_packet = ServerPacket::Message {
										player_id: *own_id,
										message: format!("&e{} has joined the server.", username),
									};
									for player in &mut data.players {
										player.packets_to_send.push(spawn_packet.clone());
										if player.id != *own_id {
											reply_queue.push_back(ServerPacket::SpawnPlayer {
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
									reply_queue.push_back(ServerPacket::Message {
										player_id: *own_id,
										message: "Welcome to the server! Enjoyyyyyy".to_string(),
									});
									reply_queue.push_back(ServerPacket::UpdateUserType {
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
									let block =
										data.level.get_block(x as usize, y as usize, z as usize);
									// check if bedrock
									if block == 0x07
										&& data
											.players
											.iter()
											.find_map(|p| {
												(p.id == *own_id).then_some(p.player_type)
											})
											.unwrap_or_default() != PlayerType::Operator
									{
										continue;
									}
									let packet = ServerPacket::SetBlock {
										x,
										y,
										z,
										block_type,
									};
									data.level
										.set_block(x as usize, y as usize, z as usize, block_type);
									for player in &mut data.players {
										player.packets_to_send.push(packet.clone());
									}
								}
								ClientPacket::PositionOrientation {
									_player_id: _,
									x,
									y,
									z,
									yaw,
									pitch,
								} => {
									let packet = ServerPacket::SetPositionOrientation {
										player_id: *own_id,
										x,
										y,
										z,
										yaw,
										pitch,
									};
									let mut data = data.write().await;
									for player in &mut data.players {
										player.packets_to_send.push(packet.clone());
									}
								}
								ClientPacket::Message { player_id, message } => {
									let mut data = data.write().await;
									println!("{message}");
									let message = format!(
										"&f<{}> {message}",
										data.players
											.iter()
											.find(|p| p.id == *own_id)
											.expect("should never fail")
											.username
									);
									let packet = ServerPacket::Message { player_id, message };
									for player in &mut data.players {
										player.packets_to_send.push(packet.clone());
									}
								}
							}
						} else {
							println!("unknown packet id: {:0x}", packet_buf[0]);
						}
					}
				}
				Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
				Err(e) => return Err(e),
			}
		}

		if ready.is_writable() {
			{
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
						reply_queue.push_back(packet);
					}
				}
			}

			while let Some(packet) = reply_queue.pop_front() {
				let writer = PacketWriter::default().write_u8(packet.get_id());
				let msg = packet.write(writer).into_raw_packet();
				stream.write_all(&msg).await?;
			}
		}
	}

	println!("remaining packets: {}", reply_queue.len());

	Ok(None)
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
