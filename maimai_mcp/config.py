from pathlib import Path

from pydantic_settings import BaseSettings, SettingsConfigDict

from .log import logger as log  # noqa: F401

Root = Path(__file__).parent


class Settings(BaseSettings):
    model_config = SettingsConfigDict(
        env_file=Root / ".env",
        env_file_encoding="utf-8",
        extra="ignore",
    )


class BaseConfig(Settings):
    maimaidx_path: str
    maimaidx_alias_proxy: bool = False
    save_in_memory: bool | None = True
    assets_online: bool | None = True
    bot_name: str = "maimai"
    default_qq: int | None = None
    default_username: str | None = None  # Diving-Fish username when no --qq
    output_dir: str = str(Root.parent / "output")


class DivingFishConfig(Settings):
    divingfish_prober_proxy: bool = False
    divingfish_token: str | None = None


class LxnsConfig(Settings):
    lxns_dev_token: str | None = None
    lx_client_id: str | None = None
    lx_client_secret: str | None = None
    redirect_uri: str | None = None


maiconfig = BaseConfig()
dfconfig = DivingFishConfig()
lxnsconfig = LxnsConfig()
