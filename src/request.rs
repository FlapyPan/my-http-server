use std::collections::BTreeMap;
use crate::constant;
use crate::error::{Fail, Result};
use crate::utils::split;

/// 支持的http方法
#[derive(Debug, PartialEq)]
pub enum HttpMethod {
    Unknown,
    Options,
    Get,
    Post,
}

// 实现字符串的into()方法
impl From<&str> for HttpMethod {
    fn from(s: &str) -> Self {
        match s {
            "OPTIONS" => HttpMethod::Options,
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            _ => HttpMethod::Unknown,
        }
    }
}

/// 支持的http版本
#[derive(Debug, PartialEq)]
pub enum HttpVersion {
    Unknown,
    V1_1,
    V2_0,
}

// 实现字符串的into()方法
impl From<&str> for HttpVersion {
    fn from(s: &str) -> Self {
        match s {
            "HTTP/1.1" => HttpVersion::V1_1,
            "HTTP/2.0" => HttpVersion::V2_0,
            _ => HttpVersion::Unknown,
        }
    }
}

/// http请求
#[derive(Debug)]
pub struct HttpRequest<'a> {
    // 请求方法
    method: HttpMethod,
    // 请求路径
    url: &'a str,
    // 请求版本
    version: HttpVersion,
    // 源ip
    ip: &'a str,
    // 请求头
    headers: BTreeMap<String, &'a str>,
    // 参数
    search_params: BTreeMap<String, &'a str>,
    // 请求体
    body: BTreeMap<String, Vec<u8>>,
}

impl<'a> HttpRequest<'a> {
    pub fn from(raw_header: &'a str,
                raw_body: Vec<u8>,
                ip: &'a str,
    ) -> Result<HttpRequest<'a>> {
        let mut header = raw_header.lines();
        // 获取请求行
        let req_ln = header.next()
            .ok_or_else(|| Fail::new("获取请求行失败"))?;
        // 按照空格分割
        let mut words = req_ln.split_whitespace();
        // 获取请求方法
        let method: HttpMethod = words.next()
            .ok_or_else(|| Fail::new("无法解析请求方法"))?.into();
        let mut search_params_raw = "";
        let url = if let Some(full_url) = words.next() {
            let mut split_url = full_url.splitn(2, '?');
            let url = split_url.next().ok_or_else(|| Fail::new("无法解析请求地址"))?;
            if let Some(params) = split_url.next() {
                search_params_raw = params;
            }
            url
        } else { "/" };
        // 获取http版本
        let version: HttpVersion = words.next()
            .ok_or_else(|| Fail::new("无法解析http协议版本"))?.into();
        // 读取请求头
        let mut headers = BTreeMap::new();
        for hl in header {
            let mut split_hl = hl.splitn(2, ":");
            if let (Some(key), Some(value)) = (split_hl.next(), split_hl.next()) {
                headers.insert(key.trim().to_lowercase(), value.trim());
            }
        }
        // 查询参数
        let search_params = parse_parameters(search_params_raw, |v| v)?;
        // 处理请求体
        let body = parse_body(&headers, &raw_body)?;
        Ok(Self {
            method,
            url,
            version,
            ip,
            headers,
            search_params,
            body,
        })
    }
    pub fn method(&self) -> &HttpMethod {
        &self.method
    }
    pub fn url(&self) -> &str {
        &self.url
    }
    pub fn version(&self) -> &HttpVersion {
        &self.version
    }
    pub fn ip(&self) -> &str {
        &self.ip
    }
    pub fn headers(&self) -> &BTreeMap<String, &'a str> {
        &self.headers
    }
    pub fn search_params(&self) -> &BTreeMap<String, &'a str> {
        &self.search_params
    }
    pub fn body(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.body
    }
    pub fn body_utf8(&self) -> BTreeMap<String, String> {
        let mut form = BTreeMap::new();
        for (k, v) in &self.body {
            form.insert(k.to_string(), String::from_utf8_lossy(v).to_string());
        }
        form
    }
}

/// 处理请求体
fn parse_body(headers: &BTreeMap<String, &str>, body: &[u8]) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut boundary = None;
    // 获取content-type
    let content_type = match headers.get("content-type") {
        None => constant::TEXT_PLAIN,
        Some(&s) => {
            let c_type = s.trim();
            for part in s.split(';') {
                let part = part.trim();
                if part.starts_with("boundary=") {
                    boundary = part.split('=').nth(1);
                }
            }
            c_type
        }
    };
    if content_type.starts_with(constant::APPLICATION_X_WWW_FORM_URLENCODED) {
        // 普通表单
        parse_parameters(&String::from_utf8(body.to_vec())?, |v| {
            v.as_bytes().to_vec()
        })
    } else if content_type.starts_with(constant::MULTIPART_FORM_DATA) {
        // Multipart表单
        parse_multipart_form(body, boundary.ok_or_else(|| Fail::new("没有有效的boundary"))?)
    } else {
        // 其他类型存储为原始字节
        let mut map = BTreeMap::new();
        map.insert(String::from("__raw"), body.to_vec());
        Ok(map)
    }
}

/// 转换MultipartForm
fn parse_multipart_form(body: &[u8], boundary: &str) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut params = BTreeMap::new();
    // 拆分
    let mut sections = split(&body, format!("--{}\r\n", boundary));
    let last_sep = format!("--{}--\r\n", boundary);
    // 去掉空的第一部分
    sections.remove(0);
    for mut section in sections {
        // 检查是否是最后一部分
        if section.ends_with(last_sep.as_bytes()) {
            // 去除最后一部分
            section = &section[..(section.len() - last_sep.len() - 2)];
        }
        let lines = split(&section, b"\r\n");
        let name = String::from_utf8_lossy(lines[0])
            .split(';')
            .map(|s| s.trim())
            .find_map(|s| {
                if s.starts_with("name=") {
                    let name = s.split('=').nth(1)?;
                    Some(name[1..(name.len() - 1)].to_lowercase())
                } else { None }
            })
            .ok_or_else(|| Fail::new("表单内容没有name属性"))?;
        // TODO 获取每个section中的content-type
        // 查找数据行
        let mut data_line_idx = 0_usize;
        for &l in &lines {
            data_line_idx += 1;
            if l.len() == 0 {
                break;
            }
        }
        // 获取值
        let data_line = lines
            .get(data_line_idx)
            .ok_or_else(|| Fail::new("表单内容损坏"))?;
        let value = data_line.to_vec();
        params.insert(name, value);
    }
    Ok(params)
}

/// 转换表单和查询参数
fn parse_parameters<'a, V>(raw: &'a str, process_value: fn(&'a str) -> V)
                           -> Result<BTreeMap<String, V>> {
    let mut params = BTreeMap::new();
    // 分割参数
    for p in raw.split('&') {
        // 分割key和value
        let mut ps = p.splitn(2, '=');
        params.insert(
            ps.next()
                .ok_or_else(|| Fail::new("损坏的参数"))?
                .trim()
                .to_lowercase(),
            process_value(if let Some(value) = ps.next() {
                value.trim()
            } else { "" }),
        );
    }
    Ok(params)
}
