from sqlalchemy import Column, Enum, MetaData, text
from sqlalchemy.ext.asyncio import create_async_engine
from sqlmodel import Field, SQLModel, select
from sqlmodel.ext.asyncio.session import AsyncSession

from ...core.clients.exceptions import UserNotBindError
from ...resources import data_dir
from ..clients.lxns.models.oauth import OAuth2Token
from ..merge.models import ServiceName, Theme

db = data_dir / "user.db"

metadata_user = MetaData()


class UserBase(SQLModel):
    __abstract__ = True
    metadata = metadata_user


class User(UserBase, table=True):
    ID: int = Field(default=None, primary_key=True, index=True, exclude=True)
    qqid: int
    friend_code: int | None = Field(default=None)
    access_token: str | None = Field(default=None)
    refresh_token: str | None = Field(default=None)
    # Diving-Fish score Import-Token (not developer token)
    import_token: str | None = Field(default=None)
    service: ServiceName = Field(
        default=ServiceName.DIVINGFISH, sa_column=Column(Enum(ServiceName))
    )
    theme: Theme = Field(default=Theme.CIRCLE, sa_column=Column(Enum(Theme)))


engine = create_async_engine(f"sqlite+aiosqlite:///{str(db)}", echo=False)


async def ensure_user_columns() -> None:
    """Add columns missing on older user.db (create_all does not alter)."""
    async with engine.begin() as connect:
        result = await connect.execute(text("PRAGMA table_info(user)"))
        cols = {row[1] for row in result.fetchall()}
        if "import_token" not in cols:
            await connect.execute(
                text("ALTER TABLE user ADD COLUMN import_token VARCHAR")
            )


async def create_database():
    async with engine.begin() as connect:
        await connect.run_sync(metadata_user.create_all)
    await ensure_user_columns()


async def get_user(qqid: int) -> User:
    async with AsyncSession(engine) as session:
        statement = select(User).where(User.qqid == qqid)
        result = await session.exec(statement)
        user = result.first()
        if user is None:
            raise UserNotBindError
        return user


async def update_user(
    qqid: int,
    *,
    friend_code: int | None = None,
    service: ServiceName | None = None,
    token: OAuth2Token | None = None,
    theme: Theme | None = None,
    import_token: str | None = None,
    clear_import_token: bool = False,
) -> User:
    update_data: dict = {
        "friend_code": friend_code,
        "service": service,
        "access_token": token.access_token if token else None,
        "refresh_token": token.refresh_token if token else None,
        "theme": theme,
    }
    if clear_import_token:
        update_data["import_token"] = None
    elif import_token is not None:
        update_data["import_token"] = import_token
    # Drop Nones except explicit clear of import_token
    cleaned = {k: v for k, v in update_data.items() if v is not None}
    if clear_import_token:
        cleaned["import_token"] = None

    async with AsyncSession(engine) as session:
        statement = select(User).where(User.qqid == qqid)
        result = await session.exec(statement)
        if user := result.first():
            user.sqlmodel_update(cleaned)
        else:
            user = User(qqid=qqid)
            user.sqlmodel_update(cleaned)
            session.add(user)
        await session.commit()
        await session.refresh(user)
        return user


async def delete_user(qqid: int) -> bool:
    async with AsyncSession(engine) as session:
        statement = select(User).where(User.qqid == qqid)
        result = await session.exec(statement)
        if user := result.first():
            await session.delete(user)
            await session.commit()
            return True
        return False
