import pytest
from dotenv import load_dotenv
import os
import deviantart_py


@pytest.fixture(scope="session", autouse=True)
def client() -> deviantart_py.Client:
    load_dotenv()

    USERNAME = os.environ["DEVIANTART_USERNAME"]
    PASSWORD = os.environ["DEVIANTART_PASSWORD"]

    client = deviantart_py.Client()

    if os.path.exists("cookies.json"):
        with open("cookies.json", "r", encoding="utf-8") as cookies_file:
            cookies = cookies_file.read()
        client.load_cookies(cookies)

    if not client.is_logged_in():
        client.login(USERNAME, PASSWORD)

        cookies = client.dump_cookies()
        with open("cookies.json", "w", encoding="utf-8") as cookies_file:
            cookies_file.write(cookies)

    return client
