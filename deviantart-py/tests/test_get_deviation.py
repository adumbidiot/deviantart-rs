import deviantart_py


def test_get_deviation(client: deviantart_py.Client) -> None:
    url = "https://www.deviantart.com/zilla774/art/chaos-gerbil-RAWR-119577071"
    client.get_deviation(url)
