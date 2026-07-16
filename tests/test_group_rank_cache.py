"""Unit tests for player-cache + group rank (no network)."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from maimai_mcp.core import player_cache
from maimai_mcp.core.qq_identity_store import (
    empty_cache,
    list_group_members,
    upsert_group_member,
    write_cache,
)
from maimai_mcp.features.group_rank.query import (
    query_group_member_rank,
    query_group_rating_rank,
    query_group_song_rank,
)


@pytest.fixture()
def isolated_caches(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    ident = tmp_path / "ident"
    scores = tmp_path / "scores"
    ident.mkdir()
    scores.mkdir()
    monkeypatch.setenv("QQ_IDENTITY_CACHE_DIR", str(ident))
    monkeypatch.setenv("PLAYER_CACHE_DIR", str(scores))
    # Clear module-level path resolution by re-reading env each call (already does).
    yield ident, scores


def test_list_group_members(isolated_caches):
    cache = empty_cache()
    upsert_group_member(
        cache,
        group_id="100",
        qq="111",
        group_name="g",
        nickname="n1",
        card="名片A",
    )
    upsert_group_member(
        cache,
        group_id="100",
        qq="222",
        group_name="g",
        nickname="n2",
        card=None,
    )
    write_cache(cache)
    members = list_group_members("100")
    assert len(members) == 2
    by_qq = {m["userId"]: m for m in members}
    assert by_qq["111"]["displayName"] == "名片A"
    assert by_qq["222"]["displayName"] == "n2"


def test_write_and_rank_rating(isolated_caches):
    cache = empty_cache()
    for qq, card in (("111", "甲"), ("222", "乙"), ("333", "丙")):
        upsert_group_member(
            cache, group_id="100", qq=qq, group_name="g", nickname=qq, card=card
        )
    write_cache(cache)

    player_cache.write_rating("111", rating=16000, name="A")
    player_cache.write_rating("222", rating=15000, name="B")
    # 333 has no cache → missing

    import asyncio

    data = asyncio.run(
        query_group_rating_rank("100", sort_order="desc", output_limit=10)
    )
    assert data["rankedCount"] == 2
    assert data["missingCount"] == 1
    assert data["rows"][0]["qq"] == "111"
    assert data["rows"][0]["rating"] == 16000
    assert data["rows"][1]["qq"] == "222"


def test_song_rank_and_member(isolated_caches, monkeypatch: pytest.MonkeyPatch):
    cache = empty_cache()
    for qq in ("111", "222"):
        upsert_group_member(
            cache, group_id="100", qq=qq, group_name="g", nickname=qq, card=qq
        )
    write_cache(cache)

    player_cache.write_song_scores(
        "111",
        song_id=834,
        song_name="Test",
        charts=[
            {
                "levelIndex": 3,
                "achievements": 100.5,
                "rating": 300,
                "dxScore": 1000,
            }
        ],
    )
    player_cache.write_song_scores(
        "222",
        song_id=834,
        song_name="Test",
        charts=[
            {
                "levelIndex": 3,
                "achievements": 99.0,
                "rating": 280,
                "dxScore": 900,
            }
        ],
    )

    import asyncio

    from maimai_mcp.features.group_rank import query as gq

    class _FakeSong:
        song_id = 834
        song_name = "Test"

    monkeypatch.setattr(gq, "resolve_song", lambda key: _FakeSong())

    board = asyncio.run(
        query_group_song_rank("100", "834", sort_by="achievements")
    )
    assert board["rankedCount"] == 2
    assert board["rows"][0]["qq"] == "111"

    member = asyncio.run(
        query_group_member_rank(group_id="100", qq="222", song="834")
    )
    assert member["found"] is True
    assert member["rank"] == 2


def test_cache_json_shape(isolated_caches):
    player_cache.write_rating("999", rating=14000, name="X", source="divingfish")
    path = Path(player_cache.cache_dir()) / "by_qq" / "999.json"
    doc = json.loads(path.read_text(encoding="utf-8"))
    assert doc["rating"]["value"] == 14000
    assert doc["qq"] == "999"
