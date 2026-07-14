from pathlib import Path

from pydantic import AliasChoices, Field
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
    output_dir: str = str(Root.parent / "output")


class DivingFishConfig(Settings):
    divingfish_prober_proxy: bool = False
    divingfish_token: str | None = None


class LxnsConfig(Settings):
    lxns_dev_token: str | None = None
    lx_client_id: str | None = None
    lx_client_secret: str | None = None
    redirect_uri: str | None = None


class IdentityConfig(Settings):
    """OneBot HTTP → local identity_cache.json (friends / group members)."""

    qq_identity_cache_dir: str | None = None
    onebot_base_url: str | None = Field(
        default=None,
        validation_alias=AliasChoices(
            "onebot_base_url", "ONEBOT_BASE_URL", "napcat_base_url", "NAPCAT_BASE_URL"
        ),
    )
    onebot_access_token: str | None = Field(
        default=None,
        validation_alias=AliasChoices(
            "onebot_access_token",
            "ONEBOT_ACCESS_TOKEN",
            "napcat_access_token",
            "NAPCAT_ACCESS_TOKEN",
        ),
    )
    qq_identity_group_delay_ms: int = 250

    def effective_onebot_base_url(self) -> str | None:
        if self.onebot_base_url and self.onebot_base_url.strip():
            return self.onebot_base_url.strip().rstrip("/")
        return None


maiconfig = BaseConfig()
dfconfig = DivingFishConfig()
lxnsconfig = LxnsConfig()
identityconfig = IdentityConfig()
