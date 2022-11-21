use async_trait::async_trait;
use ezsockets::{Server, Session, Socket};
use std::{collections::HashMap, net::SocketAddr};
use tokio::net::ToSocketAddrs;

/// Type to use for Session IDs
pub type SessionID = u32;

/// Message data to send from a server
#[derive(Clone, Debug)]
pub enum Message {
	Ping { id: SessionID },
	GetBpm { id: SessionID },
	SetBpm { id: SessionID, bpm: u8 },
	GetBattery { id: SessionID },
	SetBattery { id: SessionID, battery: f32 },
}

pub struct HrmServer {
	/// Currently connected sessions
	sessions: HashMap<SessionID, Session<SessionID, Message>>,
	/// Handle to use for communication across the server
	handle: Server<Self>,
	/// Latest session ID that has been used
	latest_id: SessionID,
	/// ID of the session that is the tracker
	tracker_id: SessionID,
	/// Current heart rate BPM
	bpm: u8,
	/// Current battery percentage
	battery: f32,
}

#[async_trait]
impl ezsockets::ServerExt for HrmServer {
	type Params = Message;
	type Session = HrmSession;

	// Incoming connection
	async fn accept(
		&mut self,
		socket: Socket,
		address: SocketAddr,
		_args: <Self::Session as ezsockets::SessionExt>::Args,
	) -> Result<Session<SessionID, Self::Params>, ezsockets::Error> {
		// Get a new ID for the session
		self.latest_id += 1;
		let id = self.latest_id;

		// Create the session and add it to the map
		let session = Session::create(
			|handle| HrmSession {
				id,
				handle,
				server: self.handle.clone(),
				// tracker: false,
			},
			id,
			socket,
		);
		self.sessions.insert(id, session.clone());
		tracing::info!("Session {} created for client connecting from {}", &id, &address);

		Ok(session)
	}

	// Session is disconnecting
	async fn disconnected(&mut self, id: <Self::Session as ezsockets::SessionExt>::ID) -> Result<(), ezsockets::Error> {
		// Reset the tracker ID if it's for the disconnected session
		if id == self.tracker_id {
			self.tracker_id = 0;
		}

		// Remove the session from the map
		assert!(
			self.sessions.remove(&id).is_some(),
			"Disconnecting session not found in session map"
		);

		tracing::info!("Session {} removed for client disconnect", &id);
		Ok(())
	}

	// Sends messages
	async fn call(&mut self, params: Self::Params) -> Result<(), ezsockets::Error> {
		match params {
			// ping -> pong
			Message::Ping { id } => {
				self.sessions
					.get(&id)
					.ok_or("Unknown session ID")?
					.text("pong".into())
					.await;
			}

			// Send current BPM
			Message::GetBpm { id } => {
				self.sessions
					.get(&id)
					.ok_or("Unknown session ID")?
					.text(self.bpm.to_string())
					.await;
			}

			// Set BPM
			Message::SetBpm { id, bpm } => {
				let session = self.sessions.get(&id).ok_or("Unknown session ID")?;

				// Make this session the tracker if there isn't one
				if self.tracker_id == 0 {
					self.tracker_id = id;
				}

				// If there is already a tracker, make sure it's this session
				if self.tracker_id == id {
					// Update the BPM and respond
					let prev_bpm = self.bpm;
					self.bpm = bpm;
					session.text("ok".into()).await;

					// Notify all other sessions of the change
					if bpm != prev_bpm {
						let sessions = self.sessions.iter().filter(|&(id, _)| *id != self.tracker_id);
						for (_, session) in sessions {
							session.text(bpm.to_string()).await;
						}
					}
				} else {
					session.text("error: a tracker is already connected".into()).await;
				}
			}

			// Send current battery
			Message::GetBattery { id } => {
				self.sessions
					.get(&id)
					.ok_or("Unknown session ID")?
					.text(format!("{:.2}", self.battery))
					.await;
			}

			// Set battery
			Message::SetBattery { id, battery } => {
				let session = self.sessions.get(&id).ok_or("Unknown session ID")?;

				// Make this session the tracker if there isn't one
				if self.tracker_id == 0 {
					self.tracker_id = id;
				}

				// If there is already a tracker, make sure it's this session
				if self.tracker_id == id {
					// Update the battery value and respond
					let prev_battery = self.battery;
					self.battery = battery;
					session.text("ok".into()).await;

					// Notify all other sessions of the change
					if battery != prev_battery {
						let sessions = self.sessions.iter().filter(|&(id, _)| *id != self.tracker_id);
						for (_, session) in sessions {
							session.text(battery.to_string()).await;
						}
					}
				} else {
					session.text("error: a tracker is already connected".into()).await;
				}
			}
		};

		Ok(())
	}
}

pub struct HrmSession {
	/// Unique ID of the session
	id: SessionID,
	/// Server this session is from
	server: Server<HrmServer>,
	/// Handle to use for communication with this session
	handle: Session<SessionID, Message>,
	// /// Whether the session is for a tracker
	// tracker: bool,
}

#[async_trait]
impl ezsockets::SessionExt for HrmSession {
	type ID = SessionID;
	type Args = ();
	type Params = Message;

	// Get the ID of the session
	fn id(&self) -> &Self::ID {
		&self.id
	}

	// Text received from client
	async fn text(&mut self, text: String) -> Result<(), ezsockets::Error> {
		let cmd = text.to_lowercase();

		match cmd.as_str() {
			// Handle setting values
			cmd if cmd.starts_with("set") => {
				let parts: Vec<&str> = cmd.split_whitespace().collect();
				match parts[1] {
					"bpm" => {
						let bpm = parts[2].parse::<u8>();
						match bpm {
							Ok(bpm) => self.server.call(Message::SetBpm { id: self.id, bpm }).await,
							Err(_) => self.handle.text("error: unknown input for bpm value".into()).await
						}
					},
					"battery" => {
						let battery = parts[2].parse::<f32>();
						match battery {
							Ok(battery) => self.server.call(Message::SetBattery { id: self.id, battery }).await,
							Err(_) => self.handle.text("error: unknown input for battery value".into()).await
						}
					},
					_ => self.handle.text("error: unknown value name".into()).await
				}
			},

			// Handle getting values
			cmd if cmd.starts_with("get") => {
				let parts: Vec<&str> = cmd.split_whitespace().collect();
				match parts[1] {
					"bpm" => self.server.call(Message::GetBpm { id: self.id }).await,
					"battery" => self.server.call(Message::GetBattery { id: self.id }).await,
					_ => self.handle.text("error: unknown value name".into()).await
				}
			}

			"ping" => self.server.call(Message::Ping { id: self.id }).await,
			_ => self.handle.text("error: unknown input".into()).await
		};

		Ok(())
	}

	// Binary data received from client
	async fn binary(&mut self, _bytes: Vec<u8>) -> Result<(), ezsockets::Error> {
		unimplemented!()
	}

	// Unused
	async fn call(&mut self, _params: Self::Params) -> Result<(), ezsockets::Error> {
		Ok(())
	}
}

/// Create and run an HRM websocket server
pub async fn run<A>(address: A) -> Result<(), ezsockets::Error>
where
	A: ToSocketAddrs,
{
	let (server, _) = ezsockets::Server::create(|handle| HrmServer {
		sessions: HashMap::new(),
		handle,
		latest_id: 0,
		tracker_id: 0,
		bpm: 0,
		battery: 0.0,
	});
	ezsockets::tungstenite::run(server, address, |_socket| async move { Ok(()) }).await
}
