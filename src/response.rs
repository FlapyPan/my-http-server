use std::collections::{BTreeMap};
use crate::constant;

/// http状态码
#[derive(Debug, PartialEq, Clone)]
pub enum HttpStatus {
    Ok,
    BadRequest,
    NotFound,
    InternalServerError,
}

/// toString
impl HttpStatus {
    fn to_str<'a>(&self) -> &'a str {
        match self {
            HttpStatus::Ok => "200 OK",
            HttpStatus::BadRequest => "400 Bad Request",
            HttpStatus::NotFound => "404 Not Found",
            HttpStatus::InternalServerError => "500 Internal Server Error",
        }
    }
}

/// http响应
#[derive(Debug, PartialEq, Clone)]
pub struct HttpResponse<'a> {
    version: &'a str,
    status: HttpStatus,
    headers: BTreeMap<&'a str, &'a str>,
    body: Option<Vec<u8>>,
}

impl<'a> Default for HttpResponse<'a> {
    fn default() -> Self {
        let mut response = Self {
            version: "HTTP/1.1",
            status: HttpStatus::Ok,
            headers: BTreeMap::new(),
            body: None,
        };
        response.headers.insert("Content-Type", constant::TEXT_PLAIN);
        response.headers.insert("server", "FlapyPan/my-http-server");
        response
    }
}

impl<'a> HttpResponse<'a> {
    pub fn new(status: HttpStatus,
               headers: Option<BTreeMap<&'a str, &'a str>>,
               body: Option<Vec<u8>>,
    ) -> HttpResponse<'a> {
        let mut response: HttpResponse<'a> = HttpResponse::default();
        response.status = status;
        match headers {
            None => {}
            Some(hs) => {
                for (k, v) in hs {
                    response.headers.insert(k, v);
                }
            }
        }
        response.body = body;
        response
    }
    pub fn not_found(body: Option<Vec<u8>>) -> HttpResponse<'a> {
        let mut response: HttpResponse<'a> = HttpResponse::default();
        response.status = HttpStatus::NotFound;
        response.headers.insert("Content-Type", constant::TEXT_HTML);
        response.body = body;
        response
    }
    fn headers(&self) -> String {
        let map = self.headers.clone();
        let mut header_string: String = "".into();
        for (k, v) in map.iter() {
            header_string = format!("{}{}:{}\r\n", header_string, k, v);
        }
        header_string
    }
    /// 转换为字节数组
    pub fn to_vec(&self) -> Vec<u8> {
        let mut vec = format!(
            "{} {}\r\n{}Content-Length: {}\r\n\r\n",
            &self.version,
            &self.status.to_str(),
            &self.headers(),
            match &self.body {
                None => 0,
                Some(b) => b.len()
            },
        ).as_bytes().to_vec();
        match &self.body {
            None => {}
            Some(b) => {
                vec.append(b.to_vec().as_mut());
            }
        };
        vec
    }
}
