use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{/*mpsc,*/Mutex};
use tokio::io::{AsyncReadExt /*, AsyncWriteExt*/};
//use std::collections::HashMap;
use std::vec::Vec;
use std::env;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::str;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create the shared state. This is how all the peers communicate.
    //
    // The server task will hold a handle to this. For every new client, the
    // `state` handle is cloned and passed into the task that processes the
    // client connection.
    let state = Arc::new(Mutex::new(Shared::new()));

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8787".to_string());

    // Bind a TCP listener to the socket address.
    //
    // Note that this is the Tokio TcpListener, which is fully async.
    let mut listener = TcpListener::bind(&addr).await?;

    println!("server running on {}", addr);

    loop {
        // Asynchronously wait for an inbound TcpStream.
        let (stream, addr) = listener.accept().await?;

        // Clone a handle to the `Shared` state for the new connection.
        let state = Arc::clone(&state);

        // Spawn our handler to be run asynchronously.
        tokio::spawn(async move {
            if let Err(e) = process(state, stream, addr).await {
                println!("an error occurred; error = {:?}", e);
            }
        });
    }
}

struct Shared {
	//rework
	peers: Vec<SocketAddr>,
}

impl Shared {
	fn new() -> Self {
		Shared {
		peers: Vec::new(),
		}
	}
}
/// Process an individual chat client
async fn process(
    state: Arc<Mutex<Shared>>,
    mut stream: TcpStream,
    addr: SocketAddr,
) -> Result<(), Box<dyn Error>> {

    // Read stream to get the username.
	//REDO_____________________________
	let mut buf = [0;256];
	let n = match stream.read(&mut buf).await {
		Ok(n) => {
			if n==0 {
				println!("socket disconnected at {}", addr);
				return Ok(());
			}
			n
		}
		Err(e) => {
			println!("error on read socket at {}: {}", addr, e);
			return Ok(());
		}
	};
	let username = str::from_utf8(&buf[0..n]).unwrap();
	//_________________________________
	//assert_eq!(username, "piloswine");
	println!("connected {} on {}", username, addr);
	
	state.lock().await.peers.push(addr);
	
	loop {
		let mut buf = [0;512];
		match stream.read(&mut buf).await {
			Ok(n) => {
				if n==0 {
					println!("{}'s socket disconnected at {}", username, addr);
					return Ok(());
				}
			}
			Err(e) => {
				println!("error on read {}'s socket at {}: {}", username, addr, e);
				return Ok(());
			}
		}
		let msg = str::from_utf8(&buf).unwrap();
		println!("From {} got: {}", username, msg);
	}
	//Ok(())
}
