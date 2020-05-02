
use tokio::stream::StreamExt;
use tokio::net::{TcpListener, TcpStream};
use std::net::SocketAddr;
use tokio::sync::{mpsc};
use std::env;
use std::error::Error;
use std::sync::Arc;
use std::string::String;
use serde_json::{Value};
use tokio::task;

mod peer;
mod server;
mod message;
mod input;
use peer::*;
use server::*;
use message::*;
use input::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create the shared server. This is how all the peers communicate.
    //
    // The server task will hold a handle to this. For every new client, the
    // `server` handle is cloned and passed into the task that processes the
    // client connection.
    let servers = Arc::new(Shared::new());

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8787".to_string());

    // Bind a TCP listener to the socket address.
    //
    // Note that this is the Tokio TcpListener, which is fully async.
    let mut listener = TcpListener::bind(&addr).await?;
    println!("server running on {}", addr);

    //for debug settin 7 servers
    for i in 0..6 {
        let name = format!("server {}", i);
        servers.add_server(Arc::new(Server::new(name))).await;
    }
    // let servers_clone = servers.clone();
    // task::spawn(async move {
    //     input_process_control(servers_clone).await;
    // });
    input_process_control(servers.clone()).await;
    println!("running {} servers", servers.len().await);

    loop {
        // Asynchronously wait for an inbound TcpStream.
        let (stream, addr) = listener.accept().await?;

        let servers_clone = servers.clone();

        // Spawn our handler to be run asynchronously.
        task::spawn(async move {
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
            match peer.next().await {
                Some(Message::Broadcast(msg)) => {
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
