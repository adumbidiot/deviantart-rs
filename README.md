# deviantart-rs
A library to interact with https://deviantart.com. It tries to uses scraping because the official api is useless.

## Testing
Tests are run with `cargo test`. 
In order to run successfully, login credentials should be placed in a `config.json` file with the following format:
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