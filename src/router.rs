use crate::handler::HelloHandler;
use crate::request::HttpRequest;
use crate::response::HttpResponse;
use super::handler::{Handler, StaticHandler};

pub struct Router;

impl Router {
    pub fn route<'a>(req: HttpRequest) -> HttpResponse<'a> {
        let route: Vec<&str> = req.url().split("/").collect();
        match route[1] {
            "hello" => {
                HelloHandler::handle(&req)
            }
            // 静态资源
            _ => StaticHandler::handle(&req)
        }
    }
}