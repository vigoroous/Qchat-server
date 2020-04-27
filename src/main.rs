use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio::io::{AsyncRead, AsyncWriteExt, AsyncReadExt};
use tokio::stream::{Stream, StreamExt};

use std::collections::HashMap;
use std::task::{Poll, Context};
use std::pin::Pin;
use std::env;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::string::String;
use std::vec::Vec;
use serde_json::json;
use serde_repr::*;
use serde::{Serialize, Deserialize};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create the shared server. This is how all the peers communicate.
    //
    // The server task will hold a handle to this. For every new client, the
    // `server` handle is cloned and passed into the task that processes the
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
    for i in 1..4 {
        let name = format!("server {}", i);
        servers.add_server(Arc::new(Server::new(name))).await;
    }
    println!("running {} servers", servers.len().await);

    let servers = Arc::new(servers);

    loop {
        // Asynchronously wait for an inbound TcpStream.
        let (stream, addr) = listener.accept().await?;

        let servers_clone = servers.clone();

        // Spawn our handler to be run asynchronously.
        tokio::spawn(async move {
            if let Err(e) = process(servers_clone, stream, addr).await {
                println!("an error occurred; error = {:?}", e);
            }
        });
    }
}

/// Shorthand for the transmit half of the message channel.
type Tx = mpsc::UnboundedSender<String>;

/// Shorthand for the receive half of the message channel.
type Rx = mpsc::UnboundedReceiver<String>;

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
    async fn add_server(&mut self, server: Arc<Server>) {
        self.servers.lock().await.push(server);
    }
    async fn choose_server(&self, pos: u32) -> Option<Arc<Server>> {
        let servers = self.servers.lock().await;
		return match servers.get(pos as usize) {
			Some(server) => Some(server.clone()),
			None => None
		};
    }
    async fn len(&self) -> usize {
        return self.servers.lock().await.len();
    }
    async fn servers_to_json_arr(&self) -> String {
        let servers = self.servers.lock().await;
		let json_vec:Vec<&str> = servers.iter().map(|x| x.name.as_str()).collect();
        return serde_json::to_string(&json_vec).unwrap();
    }
}

struct Server {
        //rework_add_tx
        name: String,
        peers: Mutex<HashMap<SocketAddr, Tx>>,
}

impl Server {
        //rework
        fn new(name: String) -> Self {
            Server {
                name: name,
                peers: Mutex::new(HashMap::new()),
            }
        }
        //rework to peer
        async fn push_new_peer(&self, addr: SocketAddr, tx: Tx) {
            //pushing address
            let mut sync_peers = self.peers.lock().await;
            sync_peers.insert(addr, tx);
            println!("pushing new peer: ({})", addr);
            println!("new length of peers vec: {}", sync_peers.len());
        }
        async fn remove_by_addr(&self, addr: &SocketAddr) {
            let mut sync_peers = self.peers.lock().await;
            sync_peers.remove(addr);
            println!("removing peer {}", addr);
            println!("new length of peers vec: {}", sync_peers.len());
        }
		/*
        async fn len(&self) -> usize {
            return self.peers.lock().await.len();
        }
		*/
        async fn broadcast(&self, sender: SocketAddr, msg: &str) {
            let mut sync_peers = self.peers.lock().await;
            for peer in sync_peers.iter_mut() {
                if *peer.0 != sender {
                    let _ = peer.1.send(msg.into());
                }
            }
        }
}

enum Message {
    Received(String),
    Broadcast(String),
}

//MAKE DUMMY PEER
struct DummyPeer {
    stream: TcpStream,
}

struct Peer {
    peer: DummyPeer,
    rx: Rx,
    //rx: Option<Rx>,
}

impl DummyPeer {
    async fn new(stream: TcpStream) -> Self {
        DummyPeer {
            stream: stream,
        }
    }
    async fn write_stream(&mut self, msg: &str) {
        self.stream.write_all(msg.as_bytes()).await.expect("failed to write to peers");
    }
    async fn read_stream(&mut self) -> Option<String> {
        let mut buf = [0;512]; //NEED TO CHANHGE THIS BUFFER
        let n = match self.stream.read(&mut buf).await {
            Ok(n) => n,
            Err(_) => {return None;},
        };
        if n==0 {return None;}
        let msg = String::from_utf8(buf[0..n].to_vec()).unwrap();
        return Some(msg);
    }
}

impl Peer {
    async fn new(server: Arc<Server>, peer: DummyPeer) -> Self {
        let addr = peer.stream.peer_addr().expect("failed to get addr");

        // Create a channel for this peer
        let (tx, rx) = mpsc::unbounded_channel();

        server.push_new_peer(addr, tx).await;
        Peer {
            peer: peer,
            rx: rx,
        }
    }
    async fn write_stream(&mut self, msg: &PeerMessage) {
		let msg = serde_json::to_string(msg).unwrap();
        self.peer.write_stream(&msg).await;
    }
    async fn _read_stream(&mut self) -> Option<String> {
        return self.peer.read_stream().await;
    }
	
	async fn change_server(&mut self, addr: SocketAddr, server_old: Arc<Server>, server_new: Arc<Server>) {
		server_old.remove_by_addr(&addr).await;
		let (tx, rx) = mpsc::unbounded_channel();
		self.rx = rx;
		server_new.push_new_peer(addr, tx).await;
        println!("moving peer {} to {}", addr, server_new.name);
	}
}

impl Stream for Peer {
//    type Item = Result<(String, usize), Box<dyn Error>>;
    type Item = Message;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        
        if let Poll::Ready(Some(v)) = Pin::new(&mut self.rx).poll_next(cx) {
            return Poll::Ready(Some(Message::Received(v)));
        }

        let mut buf = [0;512]; //NEED TO CHANHGE THIS BUFFER
        let n = match Pin::new(&mut self.peer.stream).poll_read(cx, &mut buf) {
            Poll::Ready(Ok(num_bytes_read)) => num_bytes_read,
            Poll::Ready(Err(_)) => 0,
            Poll::Pending => return Poll::Pending,
        };
        if n==0 {return Poll::Ready(None);}
        let msg = String::from_utf8(buf[0..n].to_vec()).unwrap();
        return Poll::Ready(Some(Message::Broadcast(msg)));
    }

}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug)]
#[repr(u8)]
enum MessageType {
	ServerChoice, //0
	Message, //1
}

#[derive(Serialize, Deserialize, Debug)]
struct PeerMessage {
	message_type: MessageType,
	message: String,
}

/// Process an individual chat client
async fn process(
    servers: Arc<Shared>,
    stream: TcpStream,
    addr: SocketAddr,
) -> Result<(), Box<dyn Error>> {

    //setting new dummy_peer
    let mut dummy_peer = DummyPeer::new(stream).await;

    // Read stream to get the username.
	//REDO_____________________________
        let username = dummy_peer.read_stream().await.expect("failed to get username");
	//_________________________________
	//assert_eq!(username, "piloswine");

    //debug, need to wrap in func mb________________________________
    let servers_json = &servers.servers_to_json_arr().await;
    println!("Sending servers {}", servers_json);

    dummy_peer.write_stream(servers_json).await;

	let server_choice = dummy_peer.read_stream().await.expect("failed to get choice");
	let server_choice: PeerMessage = serde_json::from_str(&server_choice).unwrap();
	println!("debug: ({:?})", server_choice);
	if server_choice.message_type != MessageType::ServerChoice {
		println!("oops got wrong message");
		return Ok(());
	}
	
	let server_choice = server_choice.message.parse::<u32>().expect("failed to parse choice");
    let server = servers.choose_server(server_choice).await.expect("failed to get server");
    println!("connecting {} on {}", username, server.name);

    let mut peer = Peer::new(server.clone(), dummy_peer).await;
    //______________________________________________________________
	println!("connected {} on {}", username, addr);

        loop {
            match peer.next().await {
                Some(Message::Broadcast(msg)) => {
					let msg: PeerMessage = serde_json::from_str(&msg).unwrap();
                    match msg.message_type {
						MessageType::Message => {
							println!("From {} got: {}; broadcasting...", username, msg.message);
							let msg = json!({"name": username, "data": msg}).to_string();
							server.broadcast(addr, &msg).await;						
						},
						MessageType::ServerChoice => {
							let server_choice = msg.message.parse::<u32>().expect("failed to parse choice");
							let server_new = servers.choose_server(server_choice).await.expect("failed to get server");
							peer.change_server(addr, server.clone(), server_new).await;
						},
					};
                },
                Some(Message::Received(msg)) => {
					let msg = PeerMessage{message_type: MessageType::Message, message: msg};
                    peer.write_stream(&msg).await;
                },
                None => {
                    server.remove_by_addr(&addr).await;
                    return Ok(());
                },
            }
	}
	//Ok(())
}
