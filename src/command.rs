use crate::player::PlayerType;

const CMD_ME: &str = "me";
const CMD_SAY: &str = "say";
const CMD_SET_PERM: &str = "setperm";
const CMD_KICK: &str = "kick";

/// enum for possible commands
#[derive(Debug, Clone)]
pub enum Command<'m> {
	/// for rp, prefixes `action` with `*<username>`
	///
	/// i.e. `/me says hello` becomes `*<username> says hello`
	Me { action: &'m str },
	/// sends a message prefixed with `[SERVER]` instead of the player's username
	Say { message: &'m str },
	/// sets permissions for a player
	SetPermissions {
		player_username: &'m str,
		permissions: PlayerType,
	},
	Kick {
		username: &'m str,
		message: Option<&'m str>,
	},
}

impl<'m> Command<'m> {
	/// the prefix for commands
	pub const PREFIX: char = '/';

	/// parses a command, returning the parsed command or an error to be displayed to the player who sent the command
	pub fn parse(input: &'m str) -> Result<Command, String> {
		let (command_name, mut arguments) = input.split_once(' ').unwrap_or((input, ""));
		Ok(match command_name {
			CMD_ME => Self::Me { action: arguments },
			CMD_SAY => Self::Say { message: arguments },
			CMD_SET_PERM => Self::SetPermissions {
				player_username: Self::next_string(&mut arguments)?,
				permissions: arguments.trim().try_into()?,
			},
			CMD_KICK => {
				let username = Self::next_string(&mut arguments)?;
				let message = arguments.trim();
				let message = (!message.is_empty()).then_some(message);
				Self::Kick { username, message }
			}
			_ => return Err(format!("Unknown command: {command_name}")),
		})
	}

	/// checks which permissions are required to run this command
	pub fn perms_required(&self) -> PlayerType {
		match self {
			Self::Me { .. } => PlayerType::Normal,
			_ => PlayerType::Moderator,
		}
	}

	/// gets the next string argument from the command
	fn next_string(args: &mut &'m str) -> Result<&'m str, String> {
		if args.is_empty() {
			return Err("Missing argument".to_string());
		}

		let (start_index, end_index, extra) = if args.starts_with('"') {
			let mut end_index = 1;
			let mut extra = 1;
			while end_index < args.len() {
				if let Some(index) = args[end_index..].find('"') {
					end_index += index;
					if &args[end_index - 1..=end_index - 1] == "\\" {
					} else {
						break;
					}
				} else {
					end_index = args.len();
					extra = 0;
					break;
				}
			}
			(1, end_index, extra)
		} else {
			(0, args.find(' ').unwrap_or(args.len()), 0)
		};

		let result = &args[start_index..end_index];
		*args = &args[end_index + extra..];

		Ok(result)
	}
}
