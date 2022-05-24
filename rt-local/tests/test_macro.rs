use rt_local::{rt_local_test, yield_now};

#[rt_local_test]
async fn test_macro() {
    yield_now().await;
}

#[rt_local_test]
#[should_panic]
async fn test_macro_panic() {
    yield_now().await;
    panic!("ok");
}
