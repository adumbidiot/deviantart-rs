class Deviation:
    id: int
    title: str
    description: str | None
    type: str
    download_url: str | None
    fullview_url: str | None
    additional_media_download_urls: list[str | None] | None

    def get_file_name(self, type: str = "download") -> str: ...
    def to_json(self, pretty: bool) -> str: ...
    @staticmethod
    def from_json(value: str) -> Deviation: ...
    def __repr__(self) -> str: ...

class Folder:
    id: int
    name: str
    owner_name: str
    deviation_ids: list[int]

    def to_json(self, pretty: bool) -> str: ...
    @staticmethod
    def from_json(value: str) -> Deviation: ...
    def __repr__(self) -> str: ...

class Client:
    def __init__(self) -> None: ...
    def get_deviation(self, deviation: str | int) -> Deviation: ...
    def download_deviation(
        self, deviation: Deviation, use_fullview: bool = False
    ) -> bytes: ...
    def is_logged_in(self) -> bool: ...
    def load_cookies(self, cookie_json_string: str) -> None: ...
    def dump_cookies(self) -> str: ...
    def login(self, username: str, password: str) -> None: ...
    def get_folder(self, url: str) -> Folder: ...
