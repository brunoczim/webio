use std::time::Duration;
use webio::{join, select, task, time::timeout, try_join};

#[webio::test]
async fn timeout_and_join() {
    let first_handle = async {
        timeout(Duration::from_millis(50)).await;
        3
    };
    let second_handle = async {
        timeout(Duration::from_millis(60)).await;
        5
    };
    let third_handle = async {
        timeout(Duration::from_millis(40)).await;
        7
    };
    let (first, second, third) =
        join!(first_handle, second_handle, third_handle);
    assert_eq!((first, second, third), (3, 5, 7));
}

#[webio::test]
async fn timeout_and_try_join_success() {
    let first_handle = async {
        timeout(Duration::from_millis(50)).await;
        Result::<u32, &str>::Ok(3)
    };
    let second_handle = async {
        timeout(Duration::from_millis(60)).await;
        Ok(5)
    };
    let third_handle = async {
        timeout(Duration::from_millis(40)).await;
        Ok(7)
    };
    let result = try_join!(first_handle, second_handle, third_handle);
    let (first, second, third) = result.unwrap();
    assert_eq!((first, second, third), (3, 5, 7));
}

#[webio::test]
async fn timeout_and_try_join_failure() {
    let first_handle = async {
        timeout(Duration::from_millis(50)).await;
        Ok(3)
    };
    let second_handle = async {
        timeout(Duration::from_millis(60)).await;
        Err("boom")
    };
    let third_handle = async {
        timeout(Duration::from_millis(40)).await;
        Ok(7)
    };
    let result = try_join!(first_handle, second_handle, third_handle);
    assert_eq!(result, Err("boom"));
}

#[webio::test]
async fn timeout_and_select() {
    let first_handle = async {
        timeout(Duration::from_millis(500)).await;
        3u32
    };
    let second_handle = async {
        timeout(Duration::from_millis(50)).await;
        5u32
    };
    let third_handle = async {
        timeout(Duration::from_millis(350)).await;
        7u32
    };
    let output = select! {
        val = first_handle => val + 10,
        val = second_handle => val + 20,
        val = third_handle => val - 5
    };
    assert_eq!(output, 25);
}

#[webio::test]
async fn timeout_and_join_with_spawn() {
    let first_handle = task::spawn(async {
        timeout(Duration::from_millis(50)).await;
        3
    });
    let second_handle = task::spawn(async {
        timeout(Duration::from_millis(60)).await;
        5
    });
    let third_handle = task::spawn(async {
        timeout(Duration::from_millis(40)).await;
        7
    });
    let (first, second, third) =
        join!(first_handle, second_handle, third_handle);
    assert_eq!((first.unwrap(), second.unwrap(), third.unwrap()), (3, 5, 7));
}

#[webio::test]
async fn timeout_and_try_join_success_with_spawn() {
    let first_handle = task::spawn(async {
        timeout(Duration::from_millis(50)).await;
        3
    });
    let second_handle = task::spawn(async {
        timeout(Duration::from_millis(60)).await;
        5
    });
    let third_handle = task::spawn(async {
        timeout(Duration::from_millis(40)).await;
        7
    });
    let result = try_join!(first_handle, second_handle, third_handle);
    let (first, second, third) = result.unwrap();
    assert_eq!((first, second, third), (3, 5, 7));
}

#[webio::test]
async fn timeout_and_select_with_spawn() {
    let first_handle = task::spawn(async {
        timeout(Duration::from_millis(500)).await;
        3u32
    });
    let second_handle = task::spawn(async {
        timeout(Duration::from_millis(50)).await;
        5u32
    });
    let third_handle = task::spawn(async {
        timeout(Duration::from_millis(350)).await;
        7u32
    });
    let output = select! {
        val = first_handle => val.unwrap() + 10,
        val = second_handle => val.unwrap() + 20,
        val = third_handle => val.unwrap() - 5
    };
    assert_eq!(output, 25);
}
