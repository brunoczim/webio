use std::time::Duration;
use webio::time::timeout;

#[webio::main]
pub async fn main2() {
    timeout(Duration::from_millis(200)).await;
}
