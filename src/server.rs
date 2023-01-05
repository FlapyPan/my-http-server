use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::error::{Fail, Result};
use crate::request::HttpRequest;
use crate::response::{HttpResponse, HttpStatus};
use crate::router::Router;

#[derive(Clone, Debug)]
pub struct HttpSettings {
    /// 最大请求头大小
    pub max_header_size: usize,
    /// 最大请求体大小
    pub max_body_size: usize,
    /// 请求头读取
    pub header_buffer: usize,
    pub body_buffer: usize,
    pub header_read_attempts: usize,
    pub body_read_attempts: usize,
}

impl HttpSettings {
    /// 默认设置
    pub fn new() -> Self {
        Self {
            max_header_size: 8192, // 8kb
            max_body_size: 8192 * 1024, // 8mb
            header_buffer: 8192,
            body_buffer: 8192,
            header_read_attempts: 3,
            body_read_attempts: 3,
        }
    }
}

pub struct Server {
    socket_addr: SocketAddr,
    http_settings: Arc<HttpSettings>,
}

impl Server {
    // 构造方法
    pub fn new(addr: &str, http_settings: HttpSettings) -> Self {
        let socket_addr = addr.parse().unwrap();
        let http_settings = Arc::new(http_settings);
        Self { socket_addr, http_settings }
    }

    // 运行
    pub async fn run(&self) -> Result<()> {
        // 监听
        let conn_listener = TcpListener::bind(self.socket_addr).await?;
        println!("Running on {}", self.socket_addr);
        loop {
            // 处理每个连接
            if let Ok((stream, address)) = conn_listener.accept().await {
                let http_settings = self.http_settings.clone();
                // 开启一个异步任务
                tokio::spawn(async move {
                    let mut stream = stream;
                    match handle_conn(&http_settings, &mut stream, address).await {
                        Ok(_) => {}
                        Err(err) => {
                            println!("{}", err);
                            write_stream(&mut stream, HttpResponse::new(HttpStatus::BadRequest, None, Some(err.to_string().as_bytes().to_vec())).to_vec()).await;
                        }
                    };
                });
            }
        }
    }
}

async fn handle_conn(http_settings: &HttpSettings,
                     mut stream: &mut TcpStream,
                     addr: SocketAddr) -> Result<()> {
    // 读取请求
    let (header, mut body) = read_head(http_settings, &mut stream).await?;
    let content_length = get_content_length(header.as_str());
    if content_length > 0 {
        read_body(&http_settings, &mut stream, &mut body, content_length).await?;
    }
    let ip = addr.ip().to_string();
    let request = HttpRequest::from(&header, body, &ip[..])?;
    let response = Router::route(request);
    write_stream(stream, response.to_vec()).await;
    Ok(())
}

/// 响应数据
async fn write_stream(stream: &mut TcpStream, content: Vec<u8>) {
    match stream.write_all(&content).await {
        Ok(_) => {
            match stream.flush().await {
                Ok(_) => {}
                Err(err) => { println!("{}", err); }
            }
        }
        Err(err) => { println!("{}", err); }
    };
}

/// 读取请求头
async fn read_head(http_settings: &HttpSettings, stream: &mut TcpStream) -> Result<(String, Vec<u8>)> {
    // 初始化缓存
    let mut header = Vec::new();
    let mut body = Vec::new();
    let mut buf = vec![0u8; http_settings.header_buffer];

    // 不停地读取流，直到读完请求头结束
    let mut read_fails = 0;
    'l: loop {
        // 检查请求头是否超过限制
        let length = stream.read(&mut buf).await?;
        if header.len() + length > http_settings.max_header_size {
            return Fail::from("请求头大小超出限制");
        }
        // 选择有效的切片
        let buf = &buf[0..length];
        // 遍历每一个字节
        'f: for (i, &b) in buf.iter().enumerate() {
            // 如果当前字节为'\r'
            if b == b'\r' {
                // 如果当前的缓冲区中剩余字节数不够3个
                if buf.len() < i + 4 {
                    // 从流中再次读取不够数量的字节
                    let mut buf_temp = vec![0u8; i + 4 - buf.len()];
                    stream.read_exact(&mut buf_temp).await?;
                    // 合并缓冲区
                    let mut buf2 = [buf, &buf_temp].concat();
                    header.append(&mut buf2);
                    // 检查请求头是否读取完毕 \n\r\n
                    if buf2[i + 1] == b'\n' && buf2[i + 2] == b'\r' && buf2[i + 3] == b'\n' {
                        break 'l;
                    } else {
                        break 'f;
                    }
                    // 如果当前的缓冲区中剩余字节数够读取3个，而且读到 \n\r\n
                } else if buf[i + 1] == b'\n' && buf[i + 2] == b'\r' && buf[i + 3] == b'\n' {
                    // 从结束的位置，分割请求头和请求体
                    let (split1, split2) = buf.split_at(i + 4);
                    header.extend_from_slice(split1);
                    body.extend_from_slice(split2);
                    break 'l;
                }
            }
            // 没有读到 \r，而且缓冲区已经读完
            if buf.len() == i + 1 {
                // 将已经读取的内容存起来
                header.extend_from_slice(buf);
            }
        }
        // 如果没有读取完毕，重试
        if length < http_settings.header_buffer {
            read_fails += 1;
            // 超出重试次数返回错误信息
            if read_fails > http_settings.header_read_attempts {
                return Fail::from("读取请求头失败");
            }
        }
    }
    // 将请求头转为utf8字符串
    Ok((String::from_utf8(header)?, body))
}

/// 从请求头获取Content-Length
fn get_content_length(head: &str) -> usize {
    let mut size: usize = 0;
    for hl in head.lines() {
        let mut split_hl = hl.splitn(2, ":");
        if let (Some(key), Some(value)) = (split_hl.next(), split_hl.next()) {
            if key.trim().to_lowercase().eq("content-length") {
                size = match value.parse::<usize>() {
                    Ok(s) => s,
                    Err(_) => 0
                };
            }
        }
    }
    size
}

/// 读取完整的body
async fn read_body(http_settings: &HttpSettings,
                   stream: &mut TcpStream,
                   body: &mut Vec<u8>,
                   content_len: usize) -> Result<()> {
    if content_len > http_settings.max_body_size {
        return Err(Fail::new("请求体大小超出限制"));
    }
    let mut read_fails = 0;
    while body.len() < content_len {
        // 计算剩下的大小
        let rest_len = content_len - body.len();
        let buf_len = if rest_len > http_settings.body_buffer {
            http_settings.body_buffer
        } else { rest_len };
        let mut buf = vec![0u8; buf_len];
        let length = match stream.read(&mut buf).await {
            Ok(len) => { len }
            Err(_) => { return Err(Fail::new("请求体读取失败")); }
        };
        buf.truncate(length);
        // 追加
        body.append(&mut buf);
        // 最多读取次数
        if length < http_settings.body_buffer {
            read_fails += 1;
            if read_fails > http_settings.body_read_attempts {
                return Fail::from("请求体读取失败");
            }
        }
    }
    Ok(())
}