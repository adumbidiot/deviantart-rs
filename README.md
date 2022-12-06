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

## References
 * https://www.deviantart.com/developers/
 * https://github.com/wix-incubator/DeviantArt-API/issues/153
 * https://github.com/mikf/gallery-dl