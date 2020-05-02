use crate::message::Message;
use crate::server::Server;
use crate::Rx;
use tokio::stream::{Stream};
use tokio::net::{TcpStream};
use std::pin::Pin;
use std::task::{Poll, Context};
use std::net::SocketAddr;
use tokio::sync::{mpsc};
use std::sync::Arc;
use tokio::prelude::*;

//MAKE DUMMY PEER
pub struct Peer {
    pub stream: TcpStream,
    pub rx: Rx,
    //rx: Option<Rx>,
}

impl Peer {
    pub async fn new(stream: TcpStream, server: Arc<Server>) -> Self {
        let addr = stream.peer_addr().expect("failed to get addr");
        // Create a channel for this peer
        let (tx, rx) = mpsc::unbounded_channel();
        server.push_new_peer(addr, tx).await;
        Peer {
            stream: stream,
            rx: rx,
        }
    }
    pub async fn write_stream(&mut self, msg: &str) {
        self.stream.write_all(msg.as_bytes()).await.expect("failed to write to peers");
    }
    pub async fn change_server(&mut self, addr: SocketAddr, server_old: Arc<Server>, server_new: Arc<Server>) {
        let tx = server_old.remove_by_addr(addr).await.unwrap();
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
        let result = futures::ready!(Pin::new(&mut self.stream).poll_read(cx, &mut buf));

        Poll::Ready( match result {
            Ok(v) => {
                if v == 0 { None }
                else {
                    let msg = String::from_utf8(buf[0..v].to_vec()).unwrap();
                    Some(Message::Broadcast(msg))
                }
            },
            Err(e) => {
                println!("error on read {:?}", e);
                None
            },
        })
    }

}
