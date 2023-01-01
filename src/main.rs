// 服务器模块
mod server;
// 请求模块
mod request;
// 响应模块
mod response;
// 路由模块
mod router;
// 处理器模块
mod handler;

use crate::server::Server;

#[tokio::main]
async fn main() {
    Server::new("127.0.0.1:8080").run().await;
}
