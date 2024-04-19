use server::Server;

mod level;
mod packet;
mod player;
mod server;

#[tokio::main]
async fn main() -> std::io::Result<()> {
	let mut server = Server::new().await?;

	server.run().await?;

	Ok(())
}
