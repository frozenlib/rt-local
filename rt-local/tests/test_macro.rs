use rt_local::test;
use rt_local::yield_now;

#[test]
async fn test_macro() {
    yield_now().await;
}

#[test]
#[should_panic]
async fn test_macro_panic() {
    yield_now().await;
    panic!("ok");
}
