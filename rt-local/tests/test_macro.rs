use rt_local::{
    runtime::core::{run, test},
    wait_for_idle,
};

#[test]
async fn test_macro() {
    wait_for_idle().await;
}

#[test]
fn test_macro_no_async() {}

#[test]
fn test_macro_no_async_no_runtime() {
    run(async {})
}

#[test]
#[should_panic]
async fn test_macro_panic() {
    wait_for_idle().await;
    panic!("ok");
}
