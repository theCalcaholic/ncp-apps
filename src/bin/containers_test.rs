extern crate ncp;
use ncp::containers;

#[tokio::main]
async fn main() {
    containers::test().await.unwrap();
}
