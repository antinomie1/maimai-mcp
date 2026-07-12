"""Alias read + local alias add (no remote vote/push)."""

from __future__ import annotations

from ...core.errors import ValidationError, handle_errors
from ...core.service import mai, update_local_alias


@handle_errors
async def query_aliases(name: str, *, by_id: bool = False) -> str:
    name = name.lower().strip()
    aliases = None
    if by_id and name.isdigit():
        aliases = mai.total_alias_list.by_id(int(name))
    else:
        aliases = mai.total_alias_list.by_alias(name)
        if not aliases and name.isdigit():
            aliases = mai.total_alias_list.by_id(int(name))
    if not aliases:
        raise ValidationError("未找到此歌曲")
    if len(aliases) != 1:
        msg = []
        for songs in aliases:
            alias_list = "\n".join(songs.alias)
            msg.append(f"ID：{songs.song_id}\n{alias_list}")
        return f"找到{len(aliases)}个相同别名的曲目：\n" + "\n======\n".join(msg)
    real_aliases = [
        a for a in aliases[0].alias if a.lower() != aliases[0].song_name.lower()
    ]
    if not real_aliases:
        return "该曲目没有别名"
    return f"该曲目有以下别名：\nID：{aliases[0].song_id}\n" + "\n".join(real_aliases)


@handle_errors
async def add_local_alias(song_id: int, alias_name: str) -> str:
    if not mai.total_list.by_id(song_id):
        raise ValidationError(f"未找到ID为「{song_id}」的曲目")
    local_exist = mai.total_alias_list.by_id(song_id)
    if local_exist and alias_name.lower() in local_exist[0].alias:
        raise ValidationError("本地别名库已存在该别名")
    ok = await update_local_alias(song_id, alias_name)
    if not ok:
        raise ValidationError("添加本地别名失败")
    return f"已成功为ID「{song_id}」添加别名「{alias_name}」到本地别名库"
