use clap::{arg, command, Parser};
use std::net::{Ipv4Addr, SocketAddr};
use tracing::metadata::LevelFilter;

mod mdns;
mod websocket;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
	/// Socket address to listen on
	#[arg(short, long, default_value_t = SocketAddr::from(([0, 0, 0, 0], 9001)))]
	listen: SocketAddr,

	/// Disables mDNS advertisement
	#[arg(short, long)]
	disable_mdns: bool,

	/// IP to advertise (via mDNS) for connecting to
	#[arg(short, long)]
	advertise_ip: Option<Ipv4Addr>,

	/// Max log level to output
	#[arg(short = 'o', long, default_value_t = LevelFilter::INFO)]
	log_level: LevelFilter,
}

#[tokio::main]
async fn main() -> Result<(), ezsockets::Error> {
	let args = Args::parse();

	// Set up tracing
	tracing_subscriber::fmt().with_max_level(args.log_level).init();

	// Advertise the server via MDNS
	if !args.disable_mdns {
		mdns::advertise(args.listen.port(), args.advertise_ip).unwrap_or_else(|err| {
			tracing::error!("Unable to advertise via mDNS: {}", err);
		});
	}

	// Run the server
	websocket::run(args.listen).await
}
