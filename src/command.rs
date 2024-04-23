use crate::player::PlayerType;

const CMD_ME: &str = "me";
const CMD_SAY: &str = "say";
const CMD_SET_PERM: &str = "setperm";
const CMD_KICK: &str = "kick";
const CMD_STOP: &str = "stop";
const CMD_HELP: &str = "help";

/// list of commands available on the server
pub const COMMANDS_LIST: &[&str] = &[CMD_ME, CMD_SAY, CMD_SET_PERM, CMD_KICK, CMD_STOP, CMD_HELP];

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
	/// kicks a player from the server
	Kick {
		username: &'m str,
		message: Option<&'m str>,
	},
	/// command to stop the server
	Stop,
	/// gets help about the given command, or about all commands if no command is given
	Help { command: Option<&'m str> },
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
			CMD_STOP => Self::Stop,
			CMD_HELP => Self::Help {
				command: (!arguments.is_empty()).then_some(arguments),
			},
			_ => return Err(format!("Unknown command: {command_name}")),
		})
	}

	/// gets the command's name
	pub fn command_name(&self) -> &'static str {
		match self {
			Self::Me { .. } => CMD_ME,
			Self::Say { .. } => CMD_SAY,
			Self::SetPermissions { .. } => CMD_SET_PERM,
			Self::Kick { .. } => CMD_KICK,
			Self::Stop => CMD_STOP,
			Self::Help { .. } => CMD_HELP,
		}
	}

	/// checks which permissions are required to run this command
	pub fn perms_required(&self) -> PlayerType {
		Self::perms_required_by_name(self.command_name())
	}

	/// checks which permissions are required to run a command by name
	pub fn perms_required_by_name(cmd: &str) -> PlayerType {
		match cmd {
			CMD_ME => PlayerType::Normal,
			CMD_STOP => PlayerType::Operator,
			CMD_HELP => PlayerType::Normal,
			_ => PlayerType::Moderator,
		}
	}

	/// gets help about the given command
	pub fn help(cmd: &str) -> Vec<String> {
		let c = |t: &str| format!("&f{}{cmd} {t}", Self::PREFIX);

		match cmd {
			CMD_ME => vec![
				c("<action>"),
				"&fDisplays an action as if you're doing it.".to_string(),
			],
			CMD_SAY => vec![
				c("<message>"),
				"&fSends a message as being from the server.".to_string(),
			],
			CMD_SET_PERM => vec![
				c("<username> <permission level>"),
				"&fSets a player's permission level.".to_string(),
			],
			CMD_KICK => vec![
				c("<username> [reason]"),
				"&fKicks a player from the server.".to_string(),
			],
			CMD_STOP => vec![
				c(""),
				"&fStops the server while saving the level.".to_string(),
			],
			CMD_HELP => vec![
				c("[command]"),
				"&fGets a list of commands or help about a command.".to_string(),
			],
			_ => vec!["&eUnknown command!".to_string()],
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
