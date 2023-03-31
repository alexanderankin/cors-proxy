mod allow_request;
mod middleware;

use std::net::SocketAddr;

use axum::{Router, routing::get};
use axum::http::StatusCode;

//noinspection HttpUrlsUsage
#[tokio::main]
async fn main() {
    let app = router();

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let server = axum::Server::bind(&addr)
        .serve(app.into_make_service());

    println!("Listening on http://{}", server.local_addr());
    tokio::spawn(async move { server.await.unwrap(); }).await.unwrap();

}

fn router() -> Router {
    Router::new()
        .route("/", get(process_request))
        .route("/*any", get(|| async { StatusCode::FORBIDDEN }))
}

async fn process_request() -> &'static str {
    "Hello, World!"
}

#[cfg(test)]
mod test {
    use std::net::SocketAddr;

    use axum::body::Body;
    use axum::http::{Request, StatusCode, Uri};
    use axum::Server;
    use hyper::client::Client;
    use tokio::sync::oneshot::{channel, Sender};
    use tokio::task;

    use crate::router;

    /// when this is dropped, the sender shuts down the server
    #[derive(Debug)]
    struct Test {
        uri: Uri,
        _send_shutdown: Sender<()>,
    }

    #[tokio::test]
    async fn test_403() {
        let context = setup_server().await;
        let req = Request::builder()
            .method("GET")
            .uri(context.uri)
            .body(Body::empty())
            .unwrap();

        let res = Client::new().request(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
    }

    async fn setup_server() -> Test {
        let any_port = SocketAddr::from(([127, 0, 0, 1], 0));
        let server = Server::bind(&any_port)
            .serve(router().into_make_service());

        let actual_addr = server.local_addr();
        let actual_port = actual_addr.port();

        let (tx, rx): (Sender<()>, _) = channel();
        task::spawn(async move {
            // start shutting down the server, but wait for a signal
            server.with_graceful_shutdown(async {
                rx.await.ok();
            }).await.unwrap();
        });

        let uri = format!("http://{}:{}/example", actual_addr.ip().to_string(), actual_port)
            .parse::<Uri>()
            .unwrap();


        Test { uri, _send_shutdown: tx }
    }
}
