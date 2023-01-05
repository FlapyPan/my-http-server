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
// 错误处理模块
mod error;
// 工具模块
mod utils;
// 常量
mod constant;

use crate::server::{HttpSettings, Server};

#[tokio::main]
async fn main() {
    let http_settings = HttpSettings::new();
    let server = Server::new("127.0.0.1:8080", http_settings);
    server.run().await.unwrap();
}
