use std::net::SocketAddr;

use anyhow::Result;
use http_body_util::Full;
use hyper::{
    body::Bytes, server::conn::Http, service::service_fn, Body, Method, Request, Response,
    StatusCode,
};
use tokio::net::TcpListener;

async fn serve_playlist(request: Request<Body>) -> hyper::Result<Response<Body>> {
    let uri = request.uri().to_string();
    if request.method() == &Method::GET {
        if uri == "/stream.m3u8" {
            let playlist = String::new();
            return Ok(Response::new(playlist.into()));
        } else if uri.starts_with("segment-") {
            return not_found();
        }
    }

    not_found()
}

fn not_found() -> hyper::Result<Response<Body>> {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::empty())
        .unwrap())
}

pub async fn run_server() -> Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (tcp_stream, _) = listener.accept().await?;

        tokio::spawn(async move {
            if let Err(error) = Http::new()
                .serve_connection(tcp_stream, service_fn(serve_playlist))
                .await
            {
                eprintln!("Error while serving HTTP connection: {}", error);
            }
        });
    }
}
