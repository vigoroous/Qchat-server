use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio::io::{AsyncRead, AsyncWriteExt, AsyncReadExt};
use tokio::stream::{Stream, StreamExt};

use std::io;
use std::collections::HashMap;
use std::task::{Poll, Context};
use std::pin::Pin;
use std::process;
use std::env;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::string::String;
use std::vec::Vec;
use std::convert::From;
use serde_json::{Value};
use serde_repr::*;
use serde::{Serialize, Deserialize};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create the shared server. This is how all the peers communicate.
    //
    // The server task will hold a handle to this. For every new client, the
    // `server` handle is cloned and passed into the task that processes the
    // client connection.
    let servers = Shared::new();

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8787".to_string());

    // Bind a TCP listener to the socket address.
    //
    // Note that this is the Tokio TcpListener, which is fully async.
    let mut listener = TcpListener::bind(&addr).await?;
    println!("server running on {}", addr);

//for debug settin 3 servers
    for i in 0..6 {
        let name = format!("server {}", i);
        servers.add_server(Arc::new(Server::new(name))).await;
    }
    println!("running {} servers", servers.len().await);

    let servers = Arc::new(servers);
        
    let servers_clone = servers.clone();

    tokio::spawn(async move {
        input_process(servers_clone).await;
    });

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

async fn input_process(servers: Arc<Shared>) {
    let stdin = io::stdin();
    loop {
        let mut buffer = String::new();
        match stdin.read_line(&mut buffer) {
            Ok(_) => (),
            Err(e) => {
                println!("{:?}", e);
                continue;
            },
        }
        let mut iter = buffer.split_whitespace();
        match iter.next().unwrap_or("") {
            "add" | "a" => {
                match iter.next() {
                    Some(v) => {
                        servers.add_server(Arc::new(Server::new(v.into()))).await;
                        // UPDATE SERVERS
                        /* to work on
                        let _msg = PeerMessage{
                            message_type:MessageType::ServersList, 
                            message:servers.servers_to_arr().await,
                        };
                        let msg = serde_json::to_string(&msg).unwrap();
                        servers.broadcast_all(&msg).await;
                        */
                    },
                    None => {
                        println!("server name missing");
                    },
                }
            },
            "servers" | "s" => {
                let servers = servers.servers.lock().await;
                println!("Available servers:");
                servers.iter().for_each(|x| println!("{}", x.name));
            },
            "Stop" | "S" => process::exit(1),
            "peers" | "p" => {
                let servers = servers.servers.lock().await;
                let mut iter = servers.iter();
                while let Some(v) = iter.next() {
                    println!("Peers for \"{}\"", v.name);
                    let peers = v.peers.lock().await;
                    peers.iter().for_each(|x| print!("{}; ", x.0));
                    println!("");
                }
            },
            "help" | "h" => {
                println!("Available commands:");
                println!("(a)dd <arg>       -print to add server, where arg - server name");
                println!("(p)eers           -print to display connected peers (curenntly on all servers)");
                println!("(s)ervers         -print to display available servers");
                println!("(S)top            -print to stop current process");
                println!("(h)elp            -print to show this message");
            },
            "" => continue,
            &_ => print!("{}", buffer),
        }
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
    async fn add_server(&self, server: Arc<Server>) {
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
    async fn servers_to_arr(&self) -> Vec<String> {
        let servers = self.servers.lock().await;
        let json_vec:Vec<String> = servers.iter().map(|x| String::from(x.name.as_str())).collect();
        return json_vec;
    }
    async fn broadcast_all(&self, msg: &str) {
        let servers = self.servers.lock().await;
        for server in servers.iter() {
            server.broadcast_all(msg).await;
        }
    }
}

#[derive(Debug)]
struct Server {
        name: String,
        peers: Mutex<HashMap<SocketAddr, Tx>>,
}

#[allow(dead_code)]
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
            //println!("new length of peers vec: {}", sync_peers.len());
        }
        async fn remove_by_addr(&self, addr: SocketAddr) -> Result<Tx, Box<dyn Error>> {
            println!("removing peer {}", addr);
            let mut sync_peers = self.peers.lock().await;
            //println!("old length of peers vec: {}", sync_peers.len());
            match sync_peers.remove(&addr) {
                Some(v) => {
                    println!("succesfully removed {}", addr);
                    return Ok(v);
                },
                None => {
                    println!("error on removing");
                    return Err("did not find peer".into());
                },
            }
            //println!("new length of peers vec: {}", sync_peers.len());
        }
        
        async fn len(&self) -> usize {
            return self.peers.lock().await.len();
        }
        async fn broadcast_all(&self, msg: &str) {
            let sync_peers = self.peers.lock().await;
            for peer in sync_peers.iter() {
                let _ = peer.1.send(msg.to_string());
                println!("debug broadcast_all() : {:?}", peer.1);
            }
        }

        async fn broadcast(&self, sender: SocketAddr, msg: &str) {
            let sync_peers = self.peers.lock().await;
            for peer in sync_peers.iter() {
                if *peer.0 != sender {
                    let _ = peer.1.send(msg.to_string());
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
    stream: TcpStream,
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
            stream: peer.stream,
            rx: rx,
        }
    }
    async fn write_stream(&mut self, msg: &str) {
        self.stream.write_all(msg.as_bytes()).await.expect("failed to write to peers");
    }
    
    async fn change_server(&mut self, addr: SocketAddr, server_old: Arc<Server>, server_new: Arc<Server>) {
        let tx = server_old.remove_by_addr(addr).await.unwrap();
        server_new.push_new_peer(addr, tx).await;
        println!("moving peer {} to {}", addr, server_new.name);
    }
}

impl Stream for Peer {
//    type Item = Result<(String, usize), Box<dyn Error>>;
    type Item = Message;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {

        let mut buf = [0;512]; //NEED TO CHANHGE THIS BUFFER
        if let Poll::Ready(v) = Pin::new(&mut self.stream).poll_read(cx, &mut buf) {
            match v {
                Ok(v) => {
                    if v == 0 {
                        return Poll::Ready(None);
                    };
                    let msg = String::from_utf8(buf[0..v].to_vec()).unwrap();
                    return Poll::Ready(Some(Message::Broadcast(msg)));
                },
                Err(e) => {
                    println!("error on read {:?}", e);
                    return Poll::Ready(None);
                },
            }
        }

        if let Poll::Ready(Some(v)) = Pin::new(&mut self.rx).poll_next(cx) {
            return Poll::Ready(Some(Message::Received(v)));
        }

        Poll::Pending
    }

}

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug)]
#[repr(u8)]
enum MessageType {
    ServerChoice, //0
    Message, //1
    ServersList,
    ServersStatus, //setup for fetching status

    Unknown,
}

impl From<u64> for MessageType {
    fn from(v: u64) -> Self {
        match v {
            0 => MessageType::ServerChoice,
            1 => MessageType::Message,
            2 => MessageType::ServersList,
            3 => MessageType::ServersStatus,
            _ => MessageType::Unknown,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct PeerMessageText {
    name: String,
    data: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct PeerMessage<T> {
    message_type: MessageType,
    message: T,
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
    println!("got name {}", username);
    //assert_eq!(username, "piloswine");

    let servers_json = PeerMessage{
        message_type: MessageType::ServersList,
        message: servers.servers_to_arr().await,
    };

    //println!("Sending servers {:?}", servers_json);
    dummy_peer.write_stream(&serde_json::to_string(&servers_json).unwrap()).await;

    let server_choice = dummy_peer.read_stream().await.expect("failed to get choice");
    let server_choice: PeerMessage<u32> = serde_json::from_str(&server_choice).unwrap();
    // println!("debug: ({:?})", server_choice);
    if server_choice.message_type != MessageType::ServerChoice {
        println!("oops got wrong message");
        return Ok(());
    }
    
    let mut server = servers.choose_server(server_choice.message).await.expect("no such server");
    let mut peer = Peer::new(server.clone(), dummy_peer).await;
    println!("connecting {} on {}", username, server.name);
    //______________________________________________________________
    // println!("connected {} on {}", username, addr);

        loop {
            println!("{} waiting on message", username);
            match peer.next().await {
                Some(Message::Broadcast(msg)) => {
                    println!("got msg {:?} on {}", msg, username);
                    let msg: Value = match serde_json::from_str(&msg) {
                        Ok(v) => v,
                        Err(e) => {
                            server.remove_by_addr(addr).await.unwrap();
                            println!("failed to parse json message {:?}", msg);
                            return Err(Box::new(e));
                        },
                    };
                    match MessageType::from(msg["message_type"].as_u64().unwrap()) {
                        MessageType::Message => {
                            let msg:PeerMessage<String> = serde_json::from_value(msg).unwrap();
                            println!("From {} got: {}; broadcasting...", username, msg.message);
                            let msg = PeerMessage{
                                message_type: MessageType::Message,
                                message: PeerMessageText{name: String::from(username.as_str()), data: msg.message},
                            };
                            let msg = serde_json::to_string(&msg).unwrap();
                            server.broadcast(addr, &msg).await;                     
                        },
                        MessageType::ServerChoice => {
                            let msg:PeerMessage<u32> = serde_json::from_value(msg).unwrap();
                            let server_new = match servers.choose_server(msg.message).await {
                                Some(v) => v,
                                None => {
                                    server.remove_by_addr(addr).await.unwrap();
                                    println!("no such server");
                                    return Err("unable to find server".into());
                                },
                            };
                            peer.change_server(addr, server.clone(), server_new.clone()).await;
                            server = server_new;
                        },
                        //*!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
                        MessageType::ServersStatus => {
                            println!("setup for future");
                            println!("but error for now");
                        },
                        MessageType::ServersList => {
                            println!("unstable behavior :{:?}", msg);
                        },
                        //*!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
                        MessageType::Unknown => {
                            println!("unknown message occured {:?}", msg);
                        },
                    };
                },
                Some(Message::Received(msg)) => {
                    println!("got msg {:?} on {}", msg, username);
                    peer.write_stream(&msg).await;
                },
                None => {
                    server.remove_by_addr(addr).await.unwrap();
                    return Ok(());
                },
            }
    }
    //Ok(())
}
