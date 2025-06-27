#[tokio::main]
async fn main() {
    let client = deviantart::Client::new();

    client
        .login("username", "password")
        .await
        .expect("failed to sign in");
}
