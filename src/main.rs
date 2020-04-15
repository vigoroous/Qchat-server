use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{/*mpsc,*/Mutex};
use tokio::io::{AsyncReadExt /*, AsyncWriteExt*/};
//use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::string::String;
use std::vec::Vec;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create the shared state. This is how all the peers communicate.
    //
    // The server task will hold a handle to this. For every new client, the
    // `state` handle is cloned and passed into the task that processes the
    // client connection.
    let mut servers = Shared::new();

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8787".to_string());

    // Bind a TCP listener to the socket address.
    //
    // Note that this is the Tokio TcpListener, which is fully async.
    let mut listener = TcpListener::bind(&addr).await?;
    println!("server running on {}", addr);

//for debug settin 3 servers
    for _i in 0..3 {servers.add_server(Arc::new(Server::new())).await;}
    println!("running {} servers", servers.len().await);

    loop {
        // Asynchronously wait for an inbound TcpStream.
        let (stream, addr) = listener.accept().await?;

        //template for future server choose
        let state = servers.choose_server().await.expect("failed to set server");

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
        servers: Mutex<Vec<Arc<Server>>>,
}

impl Shared {
        //rework
	fn new() -> Self {
		Shared {
                    servers: Mutex::new(Vec::new()),
		}
	}
        async fn add_server(self: &mut Self, server: Arc<Server>) {
            self.servers.lock().await.push(server);
        }
        async fn choose_server(self: &mut Self) -> Result<Arc<Server>, Box<dyn Error>> {
            let servers = self.servers.lock().await;
            return Ok(servers[0].clone());
        }
        async fn len(self: &Self) -> usize {
            return self.servers.lock().await.len();
        }
}

struct Server {
        //rework
        peers: Mutex<Vec<(String, SocketAddr)>>,
}

impl Server {
        //rework
        fn new() -> Self {
            Server {
                peers: Mutex::new(Vec::new()),
            }
        }
        async fn push_new_peer(self: &Self, name: &String, addr: &SocketAddr) {
            self.peers.lock().await.push((String::from(name), addr.clone()));
            println!("pushing new peer: ({}, {})", name, addr);
            println!("new length of peers vec: {}", self.len().await);
        }
        async fn remove_peer(self: &Self, name: &String) {
            let mut sync_peers = self.peers.lock().await;
            let index = sync_peers.iter().position(|(name_got, _)| name_got==name).expect("filed to find name");
            sync_peers.remove(index);
            println!("removing peer {} at {}", name, index);
            println!("new length of peers vec: {}", sync_peers.len());
        }
        async fn len(self: &Self) -> usize {
            return self.peers.lock().await.len();
        }
}

/// Process an individual chat client
async fn process(
    state: Arc<Server>,
    mut stream: TcpStream,
    addr: SocketAddr,
) -> Result<(), Box<dyn Error>> {

    // Read stream to get the username.
	//REDO_____________________________
        let (username, n) = read_msg(&mut stream, &addr).await.unwrap();
        if n==0 {return Ok(());}
	//_________________________________
	//assert_eq!(username, "piloswine");
	println!("connected {} on {}", username, addr);
	
        state.push_new_peer(&username, &addr).await;
        //println!("connected peers: {}", state.peers.len());
	
        loop {
            let (msg, n) = read_msg(&mut stream, &addr).await.unwrap();
            if n==0 {
                state.remove_peer(&username).await;
                return Ok(());
            }
            println!("From {} got: {}", username, msg);
	}
	//Ok(())
}

async fn read_msg(stream: &mut TcpStream, addr: &SocketAddr) -> Result<(String, usize), Box<dyn Error>> {
    let mut buf = [0;512];
    let n = match stream.read(&mut buf).await {
            Ok(n) => {
                    if n==0 {
                            println!("socket disconnected at {}", addr);
                            return Ok((String::from(""), 0));
                    }
                    n
            }
            Err(e) => {
                    println!("error on read socket at {}: {}", addr, e);
                    return Err(Box::new(e));
            }
    };
    let msg = String::from_utf8(buf[0..n].to_vec()).unwrap();
    return Ok((msg, n));
}
