use anyhow::{Error, Result};
use chrono::Duration;
use futures::stream::StreamExt;
use moka::future::Cache;

fn to_value(i: u16) -> String {
    format!("VAL-{}", i)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cache: Cache<u16, String> = Cache::builder()
        .time_to_live(Duration::minutes(30).to_std().unwrap())
        .build();
    let keys: Vec<u16> = (1..=100).into_iter().collect();

    tokio_stream::iter(keys.to_owned())
        .map(|key| {
            let cache_clone = cache.clone();
            tokio::spawn(async move {
                cache_clone.insert(key, to_value(key)).await;
            })
        })
        .buffer_unordered(4)
        .collect::<Vec<_>>()
        .await;

    tokio_stream::iter(keys)
        .map(|key| {
            let cache_clone = cache.clone();
            let expect = to_value(key);
            tokio::spawn(async move {
                let value = cache_clone.get(&key).await.unwrap_or_default();

                if value == expect {
                    println!(
                        "thread_id :{:?} cached value is same to expected value. value :{} expect: {}",
                        std::thread::current().id(),
                        value, expect
                    );
                }
            })
        })
        .buffer_unordered(4)
        .collect::<Vec<_>>()
        .await;

    Ok(())
}
