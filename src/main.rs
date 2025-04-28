use anyhow::{Error, Result};
use async_trait::async_trait;
use chrono::Duration;
use moka::future::Cache;
use salvo::{
    catcher::Catcher,
    conn::{Listener, tcp::TcpListener},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt().init();
    let cache: Cache<u16, String> = Cache::builder()
        .initial_capacity(100)
        .time_to_live(Duration::minutes(30).to_std().unwrap())
        .build();

    run(cache).await;

    info!("completed main func");

    Ok(())
}

async fn run(cache: Cache<u16, String>) {
    info!("creating server");

    let cache_data = CacheData { cache };
    let health = Router::with_path("healthz").get(healthz).post(cache_data);
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

#[derive(Debug, Clone, Serialize)]
struct HealthzResponseBody {
    content: String,
}

#[derive(Debug, Clone, Deserialize, Extractible)]
#[salvo(extract(default_source(from = "body", format = "json")))]
struct HealthzRequestBody {
    content: String,
}

#[derive(Debug, Clone, Default)]
struct HealthzError;

#[handler]
async fn healthz(res: &mut Response) {
    info!("healthz was called");
    let response = HealthzResponseBody {
        content: "foo".to_string(),
    };
    res.render(Json(response));
}

#[derive(Debug, Clone)]
struct CacheData {
    cache: Cache<u16, String>,
}

#[async_trait]
impl Handler for CacheData {
    async fn handle(
        &self,
        req: &mut Request,
        _depot: &mut Depot,
        res: &mut Response,
        ctrl: &mut FlowCtrl,
    ) {
        info!("cache_data handle was called");
        ctrl.skip_rest();

        let maybe_body = req.parse_json::<HealthzRequestBody>().await;

        if maybe_body.is_err() {
            let err_response = HealthzResponseBody {
                content: "failed to parse request body.".to_string(),
            };
            res.render(Json(err_response));
        }

        let value = self.cache.get(&0).await.unwrap_or_default();
        info!("get cache. value: {}", &value);

        let body = maybe_body.unwrap();
        let content = format!("{} {}", &body.content, &value);

        let _ = self.cache.insert(0, body.content.to_owned()).await;
        info!("insert cache. key: {}, value: {}", 0, &body.content);

        let response = HealthzResponseBody { content };

        res.render(Json(response));
    }
}

#[handler]
async fn handle_error(
    &self,
    _req: &Request,
    _depot: &Depot,
    res: &mut Response,
    ctrl: &mut FlowCtrl,
) {
    error!("error was occurred");
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
