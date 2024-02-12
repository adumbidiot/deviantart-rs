# deviantart-rs
A library to interact with https://deviantart.com. It tries to uses scraping because the official api is useless.

## Examples

### Sign In
Signing in is not necessary to use this library. 
However, some content cannot be accessed without a login.
```rust
#[tokio::main]
async fn main() {
    let client = deviantart::Client::new();
    
    client
        .sign_in(&config.username, &config.password)
        .await
        .expect("failed to sign in");
}
```

## Testing
Tests are run with `cargo test` from the top folder. 
In order to run successfully, login credentials should be placed in a `config.json` file in the top folder with the following format:
```json
{
    "username": "USERNAME",
    "password": "PASSWORD"
}
```
Alternatively, these credentials may be provided with the `DEVIANTART_RS_USERNAME` and `DEVIANTART_RS_PASSWORD` environment variables.

Currently, most online tests are gated behind the `--ignored` flag, as they fail on CI. 
In order to run these tests, use `cargo test -- --ignored`

## Contributing
This project is currently mostly driven by personal need, but I would be glad to accept pull requests.
Feel free to open an issue or pull request if you feel something should be changed or upgraded.
Before opening a PR, ensure `cargo test -- --ignored` runs without error after making your changes locally on your machine,
as CI is incapable of running these tests.

## References
 * https://www.deviantart.com/developers/
 * https://github.com/wix-incubator/DeviantArt-API/issues/153
 * https://github.com/mikf/gallery-dl