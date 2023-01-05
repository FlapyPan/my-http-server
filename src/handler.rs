use std::{env, fs};
use std::collections::BTreeMap;
use crate::constant;
use crate::request::{HttpMethod, HttpRequest};
use crate::response::{HttpResponse, HttpStatus};

/// handler接口
pub trait Handler {
    fn handle<'a>(req: &HttpRequest) -> HttpResponse<'a>;
    fn load_file(file_name: &str) -> Option<Vec<u8>> {
        let file_path = format!("{}/static/{}", env!("CARGO_MANIFEST_DIR"), file_name);
        fs::read(file_path).ok()
    }
}

/// 静态资源处理器
pub struct StaticHandler;

impl Handler for StaticHandler {
    fn handle<'a>(req: &HttpRequest) -> HttpResponse<'a> {
        let route: Vec<&str> = req.url().split("/").collect();
        match route[1] {
            // 访问"/"等于访问"/index.html"
            "" => {
                let mut headers = BTreeMap::new();
                headers.insert("Content-Type", constant::TEXT_HTML);
                HttpResponse::new(HttpStatus::Ok,
                                  Some(headers),
                                  Self::load_file("index.html"))
            }
            path => match Self::load_file(path) {
                Some(contents) => {
                    let mut headers = BTreeMap::new();
                    if path.ends_with(".css") {
                        headers.insert("Content-Type", constant::TEXT_CSS);
                    } else if path.ends_with(".js") {
                        headers.insert("Content-Type", constant::TEXT_JAVASCRIPT);
                    } else {
                        headers.insert("Content-Type", constant::TEXT_HTML);
                    }
                    HttpResponse::new(HttpStatus::Ok, Some(headers), Some(contents))
                }
                None => HttpResponse::not_found(Self::load_file("404.html")),
            }
        }
    }
}

pub struct HelloHandler;

impl Handler for HelloHandler {
    fn handle<'a>(req: &HttpRequest) -> HttpResponse<'a> {
        match req.method() {
            HttpMethod::Get => {
                let str = "{\"code\":200, \"msg\":\"OK\"}";
                let mut headers = BTreeMap::new();
                headers.insert("Content-Type", constant::APPLICATION_JSON);
                HttpResponse::new(HttpStatus::Ok, Some(headers), Some(str.as_bytes().to_vec()))
            }
            _ => HttpResponse::not_found(Self::load_file("404.html"))
        }
    }
}
