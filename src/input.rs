use crate::message::{PeerMessage, MessageType};
use crate::server::{Shared, Server};
use std::sync::Arc;
use std::process;
use tokio::task;
use crate::Tx;
use crate::Rx;
use tokio::sync::{mpsc};
use std::io;

pub async fn input_process_control(server: Arc<Shared>) {
	let (tx, rx) = mpsc::unbounded_channel();

	task::spawn(async move{
		control(server, rx).await;
	});

	let _blocking = task::spawn_blocking(move || {
		input_process(tx);
	});

	task::yield_now().await;
	// blocking.await.expect("input_process_control goes wrong");
}

fn input_process(tx: Tx) {
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
        tx.send(buffer).expect("failed to send control message");
	}

}

async fn control(servers: Arc<Shared>, mut rx: Rx) {
    loop {
        let command = rx.recv().await.expect("failed to recive command");
        let mut iter = command.split_whitespace();
        match iter.next().unwrap_or("") {
            "add" | "a" => {
                match iter.next() {
                    Some(v) => {
                    	//hide in Shared like "remove" | "r"
                        servers.add_server(Arc::new(Server::new(v.into()))).await;
                        // UPDATE SERVERS
                        // to work on
                        let msg = PeerMessage{
                            message_type:MessageType::ServersList, 
                            message:servers.servers_to_arr().await,
                        };
                        let msg = serde_json::to_string(&msg).unwrap();
                        println!("adding new server: {}", v);
                        servers.broadcast_all(&msg).await;
                        
                    },
                    None => {
                        println!("server name missing");
                    },
                }
            },
            "remove" | "r" => {
                match iter.next() {
                    Some(v) => {
                    	servers.remove_server_by_name(v.to_string()).await;
                        //remake__________________________________________
                        let msg = PeerMessage{
                            message_type:MessageType::ServersList, 
                            message:servers.servers_to_arr().await,
                        };
                        let msg = serde_json::to_string(&msg).unwrap();
                        servers.broadcast_all(&msg).await;
                        //________________________________________________
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
                println!("(r)emove <arg>    -print to remove server, where arg - server name");
                println!("(s)ervers         -print to display available servers");
                println!("(S)top            -print to stop current process");
                println!("(h)elp            -print to show this message");
            },
            "" => continue,
            _v => print!("{}", _v),
        }
    }
}
