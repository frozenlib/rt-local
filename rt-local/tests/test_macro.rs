use rt_local::{runtime::core::test, wait_for_idle};

#[test]
async fn test_macro() {
    wait_for_idle().await;
}

#[test]
#[should_panic]
async fn test_macro_panic() {
    wait_for_idle().await;
    panic!("ok");
}
