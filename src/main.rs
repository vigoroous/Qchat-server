use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{/*mpsc,*/Mutex};
use tokio::io::{AsyncRead};
use tokio::stream::{Stream, StreamExt};
//use std::collections::HashMap;
use std::task::{Poll, Context};
use std::pin::Pin;
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
        //rework_add_tx
        peers: Mutex<Vec<SocketAddr>>,
}

impl Server {
        //rework
        fn new() -> Self {
            Server {
                peers: Mutex::new(Vec::new()),
            }
        }
        //rework to peer
        async fn push_new_peer(self: &Self, addr: SocketAddr) {
            //pushing address
            let mut sync_peers = self.peers.lock().await;
            sync_peers.push(addr);
            println!("pushing new peer: ({})", addr);
            println!("new length of peers vec: {}", sync_peers.len());
        }
        async fn remove_by_addr(self: &Self, addr: &SocketAddr) {
            let mut sync_peers = self.peers.lock().await;
            let index = sync_peers.iter().position(|addr_got| addr_got==addr).expect("filed to find peer");
            sync_peers.remove(index);
            println!("removing peer {} at {}", addr, index);
            println!("new length of peers vec: {}", sync_peers.len());
        }
        async fn len(self: &Self) -> usize {
            return self.peers.lock().await.len();
        }
}

struct Peer {
    stream: TcpStream,
    //add rx,
}

impl Peer {
    async fn new(state: Arc<Server>, stream: TcpStream) -> Self {
        let addr = stream.peer_addr().expect("failed to get addr");
        state.push_new_peer(addr).await;
        Peer {
            stream: stream,
        }
    }
}

impl Stream for Peer {
//    type Item = Result<(String, usize), Box<dyn Error>>;
    type Item = (String, usize);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut buf = [0;512];
        let n = match Pin::new(&mut self.stream).poll_read(cx, &mut buf) {
            Poll::Ready(Ok(num_bytes_read)) => num_bytes_read,
            Poll::Ready(Err(_)) => 0,
            Poll::Pending => return Poll::Pending,
        };
        if n==0 {return Poll::Ready(None);}
        let msg = String::from_utf8(buf[0..n].to_vec()).unwrap();
        return Poll::Ready(Some((msg, n)));
    }

}


/// Process an individual chat client
async fn process(
    state: Arc<Server>,
    mut stream: TcpStream,
    addr: SocketAddr,
) -> Result<(), Box<dyn Error>> {

        //setting new peer
        let mut peer = Peer::new(state.clone(), stream).await;

        // Read stream to get the username.
	//REDO_____________________________
        let (username, _n) = match peer.next().await {
            Some((msg, n)) => (msg, n),
            None => {
                state.remove_by_addr(&addr).await;
                return Ok(());
            },
        };
	//_________________________________
	//assert_eq!(username, "piloswine");

	println!("connected {} on {}", username, addr);

        //to-do: setting event stream

        loop {
            let (msg, _n) = match peer.next().await {
                Some((msg, n)) => (msg, n),
                None => {
                    state.remove_by_addr(&addr).await;
                    return Ok(());
                },
            };
            println!("From {} got: {}", username, msg);
	}
	//Ok(())
}
