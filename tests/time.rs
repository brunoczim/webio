use std::time::Duration;
use webio::time::{interval, timeout, Instant};

#[webio::test]
async fn timeout_and_instant() {
    let then = Instant::now();
    let time = Duration::from_millis(100);
    timeout(time).await;
    let passed = then.elapsed();
    assert!(passed >= time);
    assert!(passed < time + Duration::from_millis(50));
}

#[webio::test]
async fn interval_and_instant() {
    let time = Duration::from_millis(100);
    let handle = interval(time);
    let then = Instant::now();

    handle.tick().await;
    let passed = then.elapsed();
    assert!(passed >= time - Duration::from_millis(50));
    assert!(passed < time + Duration::from_millis(50));

    handle.tick().await;
    let passed = then.elapsed();
    assert!(passed >= time * 2 - Duration::from_millis(50));
    assert!(passed < time * 2 + Duration::from_millis(50));

    handle.tick().await;
    let passed = then.elapsed();
    assert!(passed >= time * 3 - Duration::from_millis(50));
    assert!(passed < time * 3 + Duration::from_millis(50));
}

/*
 * TODO
#[webio::test]
#[should_panic]
async fn panic_even_after_timeout() {
    let time = Duration::from_millis(100);
    timeout(time).await;
    panic!("This test should panic");
}
*/
