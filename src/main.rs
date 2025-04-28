use anyhow::{Error, Result};
use async_trait::async_trait;
use chrono::Duration;
use futures::stream::StreamExt;
use moka::future::Cache;
use salvo::{
    catcher::Catcher,
    conn::{
        Listener,
        tcp::{TcpAcceptor, TcpListener},
    },
    prelude::*,
};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tracing::info;

fn to_value(i: u16) -> String {
    format!("VAL-{}", i)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt().init();
    // let cache: Cache<u16, String> = Cache::builder()
    //     .time_to_live(Duration::minutes(30).to_std().unwrap())
    //     .build();
    // let keys: Vec<u16> = (1..=100).into_iter().collect();

    // tokio_stream::iter(keys.to_owned())
    //     .map(|key| {
    //         let cache_clone = cache.clone();
    //         let value = to_value(key);
    //         tokio::spawn(async move {
    //             println!(
    //                 "thread_id :{:?} insert to cache. key :{} value: {}",
    //                 std::thread::current().id(),
    //                 key,
    //                 value
    //             );
    //             cache_clone.insert(key, value).await;
    //         })
    //     })
    //     .buffer_unordered(4)
    //     .collect::<Vec<_>>()
    //     .await;

    // tokio_stream::iter(keys)
    //     .map(|key| {
    //         let cache_clone = cache.clone();
    //         let expect = to_value(key);
    //         tokio::spawn(async move {
    //             let value = cache_clone.get(&key).await.unwrap_or_default();

    //             if value == expect {
    //                 println!(
    //                     "thread_id :{:?} cached value is same to expected value. value :{} expect: {}",
    //                     std::thread::current().id(),
    //                     value, expect
    //                 );
    //             }
    //         })
    //     })
    //     .buffer_unordered(4)
    //     .collect::<Vec<_>>()
    //     .await;

    run().await;

    Ok(())
}

async fn run() {
    info!("creating server");
    let health = Router::with_path("healthz").get(healthz).post(post_healthz);
    let router = Router::new().push(health);

    let service = Service::new(router).catcher(Catcher::default().hoop(handle_error));

    let accepter = TcpListener::new(("0.0.0.0", 8080)).bind().await;

    let (tx, rx) = oneshot::channel();

    let server = Server::new(accepter).serve_with_graceful_shutdown(
        service,
        async {
            rx.await.ok();
            info!("received ctrl-c so shutdown.");
        },
        None,
    );

    tokio::spawn(server);

    let quit_key = tokio::signal::ctrl_c().await;
    if quit_key.is_ok() {
        let _ = tx.send(());
    }
}

#[handler]
async fn healthz(res: &mut Response) {
    info!("healthz was called");
    let response = HealthzResponseBody {
        content: "foo".to_string(),
    };
    res.render(Json(response));
}

#[derive(Debug, Clone, Serialize)]
struct HealthzResponseBody {
    content: String,
}

#[derive(Debug, Clone, Deserialize, Extractible)]
#[salvo(extract(default_source(from = "body", format = "json")))]
struct HealthzRequestBody {
    content: String,
}

#[derive(Debug, Clone)]
struct HealthzError;

#[async_trait]
impl Writer for HealthzError {
    async fn write(mut self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        let status_code = StatusCode::INTERNAL_SERVER_ERROR;
        let content = "error occurred".to_string();
        let response = HealthzResponseBody { content };
        res.status_code(status_code);
        res.render(Json(response));
    }
}

#[handler]
async fn post_healthz(
    body: HealthzRequestBody,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
) -> Result<(), HealthzError> {
    info!("post_healthz was called");
    ctrl.skip_rest();

    let content = body.content;
    let response = HealthzResponseBody { content };

    res.render(Json(response));

    Ok(())
}

#[handler]
async fn handle_error(
    &self,
    _req: &Request,
    _depot: &Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
) {
    info!("error was occurred");
    let status = res.status_code.unwrap_or_default();
    let content = match status {
        StatusCode::BAD_REQUEST => "invalid parameter is set.".to_string(),
        StatusCode::INTERNAL_SERVER_ERROR => "internal error occurred.".to_string(),
        s if s.is_client_error() => "4xx error occurred.".to_string(),
        s if s.is_server_error() => "5xx error occurred.".to_string(),
        s if s.is_informational() || s.is_success() || s.is_redirection() => String::default(),
        _ => "something went wrong.".to_string(),
    };

    let response = HealthzResponseBody { content };

    if status.is_client_error() || status.is_server_error() {
        res.status_code(status);
        res.render(Json(response));
        ctrl.skip_rest();
    }
}
