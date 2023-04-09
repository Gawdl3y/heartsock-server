use async_trait::async_trait;
use ezsockets::{Server, Session, Socket};
use std::{collections::HashMap, fmt::Display, net::SocketAddr};
use tokio::net::ToSocketAddrs;

/// Type to use for Session IDs
pub type SessionID = u32;

/// Message data to send from a server
#[derive(Clone, Debug)]
pub enum Message {
	Ping { id: SessionID },
	GetVal { id: SessionID, key: String },
	SetVal { id: SessionID, key: String, val: u8 },
}

pub struct HeartsockServer {
	/// Currently connected sessions
	sessions: HashMap<SessionID, Session<SessionID, Message>>,
	/// Handle to use for communication across the server
	handle: Server<Self>,
	/// Latest session ID that has been used
	latest_id: SessionID,
	/// ID of the session that is the tracker
	tracker_id: SessionID,
	/// Current tracked values
	values: HashMap<String, u8>,
}

#[async_trait]
impl ezsockets::ServerExt for HeartsockServer {
	type Call = Message;
	type Session = HeartsockSession;

	// Incoming connection
	async fn on_connect(
		&mut self,
		socket: Socket,
		address: SocketAddr,
		_args: <Self::Session as ezsockets::SessionExt>::Args,
	) -> Result<Session<SessionID, Self::Call>, ezsockets::Error> {
		// Get a new ID for the session
		self.latest_id += 1;
		let id = self.latest_id;

		// Create the session and add it to the map
		let session = Session::create(
			|handle| HeartsockSession {
				id,
				handle,
				server: self.handle.clone(),
			},
			id,
			socket,
		);
		self.sessions.insert(id, session.clone());
		tracing::info!("Session {} created for client connecting from {}", &id, &address);

		// Send the current values
		for (key, val) in &self.values {
			session.text(format!("{}: {}", key, val));
		}

		Ok(session)
	}

	// Session is disconnecting
	async fn on_disconnect(
		&mut self,
		id: <Self::Session as ezsockets::SessionExt>::ID,
	) -> Result<(), ezsockets::Error> {
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

	// Sends messages to connected sessions
	async fn on_call(&mut self, call: Self::Call) -> Result<(), ezsockets::Error> {
		match call {
			// ping -> pong
			Message::Ping { id } => self
				.sessions
				.get(&id)
				.ok_or("unknown session ID")?
				.text("pong".to_owned()),

			Message::GetVal { id, key } => self.sessions.get(&id).ok_or("unknown session ID")?.text(format!(
				"{}: {}",
				key,
				self.values.get(&key).expect("unknown value key")
			)),

			Message::SetVal { id, key, val } => {
				let session = self.sessions.get(&id).ok_or("unknown session ID")?;

				// Make this session the tracker if there isn't one
				if self.tracker_id == 0 {
					self.tracker_id = id;
				}

				// If there is already a tracker, make sure it's this session
				if self.tracker_id == id {
					// Update the value and respond
					let prev = self
						.values
						.insert(key.clone(), val)
						.unwrap_or_else(|| panic!("no old value for key {}", key));
					session.text("ok".to_owned());

					// Notify all other sessions of the change
					if prev != val {
						let sessions = self.sessions.iter().filter(|&(id, _)| *id != self.tracker_id);
						for (_, session) in sessions {
							session.text(format!("{}: {}", key, val));
						}
					}
				} else {
					session.text("error: a tracker is already connected".to_owned());
				}
			}
		};

		Ok(())
	}
}

pub struct HeartsockSession {
	/// Unique ID of the session
	id: SessionID,
	/// Server this session is from
	server: Server<HeartsockServer>,
	/// Handle to use for communication with this session
	handle: Session<SessionID, Message>,
}

#[async_trait]
impl ezsockets::SessionExt for HeartsockSession {
	type ID = SessionID;
	type Args = ();
	type Call = Message;

	// Get the ID of the session
	fn id(&self) -> &Self::ID {
		&self.id
	}

	// Text received from client
	async fn on_text(&mut self, text: String) -> Result<(), ezsockets::Error> {
		let cmd = text.to_lowercase();

		match cmd.as_str() {
			// Handle setting values
			cmd if cmd.starts_with("set") => {
				let parts: Vec<&str> = cmd.split_whitespace().collect();
				let key = parts[1];

				if matches!(key, "bpm" | "battery") {
					let val = parts[2].parse::<u8>();
					match val {
						Ok(val) => self.server.call(Message::SetVal {
							id: self.id,
							key: key.to_owned(),
							val,
						}),
						Err(_) => self.handle.text(format!("error: unknown input for {} value", key)),
					}
				} else {
					self.handle.text("error: unknown value key".to_owned())
				}
			}

			// Handle getting values
			cmd if cmd.starts_with("get") => {
				let parts: Vec<&str> = cmd.split_whitespace().collect();
				let key = parts[1];
				self.server.call(Message::GetVal {
					id: self.id,
					key: key.to_owned(),
				})
			}

			"ping" => self.server.call(Message::Ping { id: self.id }),
			_ => self.handle.text("error: unknown input".to_owned()),
		}

		Ok(())
	}

	// Binary data received from client
	async fn on_binary(&mut self, _bytes: Vec<u8>) -> Result<(), ezsockets::Error> {
		unimplemented!()
	}

	// Unused
	async fn on_call(&mut self, _call: Self::Call) -> Result<(), ezsockets::Error> {
		Ok(())
	}
}

/// Create and run a Heartsock websocket server
pub async fn run<A>(address: A) -> Result<(), ezsockets::Error>
where
	A: ToSocketAddrs + Display,
{
	tracing::info!("WebSocket server starting on {}", address);
	let (server, _) = ezsockets::Server::create(|handle| HeartsockServer {
		sessions: HashMap::new(),
		handle,
		latest_id: 0,
		tracker_id: 0,
		values: HashMap::from([("bpm".to_owned(), 0), ("battery".to_owned(), 0)]),
	});
	ezsockets::tungstenite::run(server, address, |_socket| async move { Ok(()) }).await
}
