# deviantart-rs
A library to interact with https://deviantart.com.
It tries to uses scraping because the official API is useless.
This primarily implements deviation downloading, and some searching.
If you require uploading, please feel free to open an issue.

## Examples

### Sign In
Signing in is not necessary to use this library. 
However, some content cannot be accessed without a login.
```rust
#[tokio::main]
async fn main() {
    let client = deviantart::Client::new();

    client
        .sign_in("username", "password")
        .await
        .expect("failed to sign in");
}
```

## Python Binding
This repository also contains a Python binding.
It is a slightly higher-level API than the Rust library,
so it may be easier to use.

### Examples

#### Download Deviation
```python
import deviantart_py
import os

client = deviantart_py.Client()

# Most deviations require a login to download,
# even ones that aren't NSFW.
# Make sure to provide one for the best results!

# Here we load from environment variables.
# You should set them to properly run this example.
#
# Look up what environment variables are if you are unfamiliar.
username = os.environ.get('USER')
password = os.environ.get('PASSWORD')

# First, try to use our saved cookies to avoid logging in.
# Spamming logins is a great way to get detected.
if os.path.exists('cookies.json'):
    print('Loading cookies...')
    with open('cookies.json', 'r', encoding='utf-8') as cookies_file:
        cookies = cookies_file.read()
    client.load_cookies(cookies)

# If the cookies didn't makes use logged in, try a new login.
if not client.is_logged_in() and username is not None and password is not None:
    print('Logging in...')
    client.login(username, password)
    
    # Save the cookies for next time.
    print('Saving cookies...')
    cookies = client.dump_cookies()
    with open('cookies.json', 'w', encoding='utf-8') as cookies_file:
        cookies_file.write(cookies)

# Keep in mind not all deviations can be downloaded.
deviation = client.get_deviation('https://www.deviantart.com/zilla774/art/chaos-gerbil-RAWR-119577071')

file_name = deviation.get_file_name(type='download')
deviation_bytes = client.download_deviation(deviation)
with open(file_name, 'wb') as output_file:
    output_file.write(deviation_bytes)
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
In order to run these tests, use `cargo test -- --ignored`.

## Contributing
This project is currently mostly driven by personal need, but I would be glad to accept pull requests.
Feel free to open an issue or pull request if you feel something should be changed or upgraded.
Before opening a PR, ensure `cargo test -- --ignored` runs without error after making your changes locally on your machine,
as CI is incapable of running these tests.

## References
 * https://www.deviantart.com/developers/
 * https://github.com/wix-incubator/DeviantArt-API/issues/153
 * https://github.com/mikf/gallery-dl