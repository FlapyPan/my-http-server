use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::io;
use tokio::io::AsyncReadExt;

pub struct Server {
    socket_addr: SocketAddr,
}

impl Server {
    // 构造方法
    pub fn new(addr: &str) -> Self {
        let socket_addr = addr.parse().unwrap();
        Self { socket_addr }
    }

    // 运行
    pub async fn run(&self) {
        // 监听
        let conn_listener = TcpListener::bind(self.socket_addr).await.unwrap();
        println!("Running on {}", self.socket_addr);
        loop {
            // 处理每个连接
            match conn_listener.accept().await {
                Ok((stream, _)) => {
                    let req_str = handle_conn(stream).await.unwrap();
                    println!("{}", req_str);
                    // TODO 调用router处理
                }
                Err(err) => { println!("{}", err); }
            };
        }
    }
}

async fn handle_conn(mut stream: TcpStream) -> io::Result<String> {
    // 一次性读1mb
    let mut all_bytes = vec![0_u8; 1024 * 1024];
    let size = stream.read(&mut all_bytes).await?;
    let req_str = String::from_utf8_lossy(&all_bytes[..size]).to_string();
    Ok(req_str)
}