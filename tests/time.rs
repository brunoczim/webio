use std::time::Duration;
use webio::time::{timeout, Instant};

#[webio::test]
async fn timeout_and_instant() {
    let then = Instant::now();
    let time = Duration::from_millis(100);
    timeout(time).await;
    let passed = then.elapsed();
    assert!(passed >= time);
    assert!(passed < time + Duration::from_millis(50));
}
