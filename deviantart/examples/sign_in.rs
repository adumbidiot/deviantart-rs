#[tokio::main]
async fn main() {
    let client = deviantart::Client::new();

    client
        .sign_in("username", "password")
        .await
        .expect("failed to sign in");
}
