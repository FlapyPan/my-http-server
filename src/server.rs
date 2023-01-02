use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::error::{Fail, Result};

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
                    handle_conn(&http_settings, stream, address).await.ok();
                });
            }
        }
    }
}

async fn handle_conn(http_settings: &HttpSettings,
                     mut stream: TcpStream,
                     _: SocketAddr) -> Result<()> {
    // 读取请求
    let http_response = match read_stream(http_settings, &mut stream).await {
        Ok((header, body)) => {
            println!("请求头: {}", header);
            println!("请求体: {}", String::from_utf8(body)?);
            // TODO 构建请求结构体、调用handler
            format!("HTTP/1.1 400\r\ncontent-type: plain/text; charset=utf-8\r\n\r\n{}", header).as_bytes().to_vec()
        }
        Err(err) => {
            format!("HTTP/1.1 400\r\ncontent-type: plain/text; charset=utf-8\r\n\r\n{}", err.to_string()).as_bytes().to_vec()
        }
    };
    // 响应数据
    stream.write_all(&http_response).await?;
    stream.flush().await?;
    Ok(())
}

async fn read_stream(http_settings: &HttpSettings, stream: &mut TcpStream) -> Result<(String, Vec<u8>)> {
    // 初始化缓存
    let mut header = Vec::new();
    let mut body = Vec::new();
    let mut buf = vec![0u8; http_settings.header_buffer];

    // 不停地读取流，直到结束
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
        // TODO 检查请求头的Content-length来确认请求体是否读完
    }
    // 将请求头转为utf8字符串
    Ok((match String::from_utf8(header) {
        Ok(header) => header,
        Err(err) => return Fail::from(err),
    }, body))
}