use crate::Tx;
use tokio::sync::{Mutex};
use std::sync::Arc;
use std::string::String;
use std::vec::Vec;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::error::Error;


pub struct Shared {
        //rework
        pub servers: Mutex<Vec<Arc<Server>>>,
}

impl Shared {
        //rework
    pub fn new() -> Self {
        Shared {
            servers: Mutex::new(Vec::new()),
        }
    }
    pub async fn add_server(&self, server: Arc<Server>) {
        self.servers.lock().await.push(server);
    }
    pub async fn choose_server(&self, pos: u32) -> Option<Arc<Server>> {
        let servers = self.servers.lock().await;
        return match servers.get(pos as usize) {
            Some(server) => Some(server.clone()),
            None => None
        };
    }
    pub async fn len(&self) -> usize {
        return self.servers.lock().await.len();
    }
    pub async fn servers_to_arr(&self) -> Vec<String> {
        let servers = self.servers.lock().await;
        let json_vec:Vec<String> = servers.iter().map(|x| String::from(x.name.as_str())).collect();
        return json_vec;
    }
    pub async fn broadcast_all(&self, msg: &str) {
        let servers = self.servers.lock().await;
        for server in servers.iter() {
            server.broadcast_all(msg).await;
        }
    }
}

#[derive(Debug)]
pub struct Server {
        pub name: String,
        pub peers: Mutex<HashMap<SocketAddr, Tx>>,
}

#[allow(dead_code)]
impl Server {
        //rework
        pub fn new(name: String) -> Self {
            Server {
                name: name,
                peers: Mutex::new(HashMap::new()),
            }
        }
        //rework to peer
        pub async fn push_new_peer(&self, addr: SocketAddr, tx: Tx) {
            //pushing address
            let mut sync_peers = self.peers.lock().await;
            sync_peers.insert(addr, tx);
            println!("pushing new peer: ({})", addr);
            //println!("new length of peers vec: {}", sync_peers.len());
        }
        pub async fn remove_by_addr(&self, addr: SocketAddr) -> Result<Tx, Box<dyn Error>> {
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
        
        pub async fn len(&self) -> usize {
            return self.peers.lock().await.len();
        }
        pub async fn broadcast_all(&self, msg: &str) {
            let sync_peers = self.peers.lock().await;
            for peer in sync_peers.iter() {
                let _ = peer.1.send(msg.to_string());
            }
        }

        pub async fn broadcast(&self, sender: SocketAddr, msg: &str) {
            let sync_peers = self.peers.lock().await;
            for peer in sync_peers.iter() {
                if *peer.0 != sender {
                    let _ = peer.1.send(msg.to_string());
                }
            }
        }
}
