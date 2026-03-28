#![recursion_limit = "1024"]

#[cfg(feature = "ssr")]
mod server;

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    server::run().await;
}

#[cfg(not(feature = "ssr"))]
pub fn main() {}
