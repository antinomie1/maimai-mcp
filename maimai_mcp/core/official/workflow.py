#!/usr/bin/env python3
"""QR → 官服成绩 → convert → 水鱼/落雪上传。"""

from __future__ import annotations

import json
import os
import re
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable

try:
    import requests
except ImportError as exc:  # pragma: no cover
    raise SystemExit(
        "missing dependency: requests. Install with: python -m pip install requests"
    ) from exc

from .client import (
    DEFAULT_KEYCHIP_ID,
    DEFAULT_PLACE_ID,
    DEFAULT_REGION_ID,
    MaimaiOfficialClient,
)
from .convert import convert_file
from .protocol import (
    ChimeSessionError,
    OfficialProtocolError,
    OfficialTitleServerError,
)

DEFAULT_DIVING_FISH_URL = (
    "https://www.diving-fish.com/api/maimaidxprober/player/update_records"
)
DEFAULT_OUTPUT_DIR = Path.home() / ".cache" / "maimai-record-imports"
QQ_RE = re.compile(r"^\d{5,12}$")


class WorkflowError(RuntimeError):
    pass


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def mask_secret(value: str) -> str:
    if len(value) <= 8:
        return "*" * len(value)
    return f"{value[:3]}...{value[-4:]}"


def normalize_qq(value: Any) -> str:
    qq = str(value or "").strip()
    if not QQ_RE.fullmatch(qq):
        raise WorkflowError("QQ 号格式不正确。")
    return qq


def normalize_token(value: Any) -> str:
    token = str(value or "").strip()
    if not token or any(ch.isspace() for ch in token):
        raise WorkflowError("Import-Token 不能为空，也不能包含空白字符。")
    return token


def output_dir_from_env() -> Path:
    return Path(
        os.environ.get("MAIMAI_UPDATE_RECORDS_OUTPUT_DIR") or DEFAULT_OUTPUT_DIR
    ).expanduser()


async def bind_import_token(qq: Any, import_token: Any) -> dict[str, Any]:
    from maimai_mcp.core.database.qq import update_user

    qq_text = normalize_qq(qq)
    token = normalize_token(import_token)
    await update_user(int(qq_text), import_token=token)
    return {
        "ok": True,
        "qq": qq_text,
        "tokenPreview": mask_secret(token),
        "import_token_bound": True,
        "import_token_preview": mask_secret(token),
        "text": f"已绑定 QQ {qq_text} 的水鱼成绩导入 token（{mask_secret(token)}）。",
    }


async def get_import_token(qq: Any) -> str:
    from maimai_mcp.core.clients.exceptions import UserNotBindError
    from maimai_mcp.core.user import resolve_user

    qq_text = normalize_qq(qq)
    try:
        user = await resolve_user(int(qq_text), auto_create=False)
    except UserNotBindError as exc:
        raise WorkflowError(
            "本地尚无该 QQ 的用户记录。"
            "请先绑定水鱼 Import-Token："
            "CLI `maimai user bind-import --qq <QQ> --token <Import-Token>`，"
            "或 MCP `maimai_user_bind_import_token`。"
            "Import-Token 在水鱼网页「编辑个人资料」生成，不是 DIVINGFISH_TOKEN。"
        ) from exc
    token = getattr(user, "import_token", None)
    if not token:
        raise WorkflowError(
            "尚未绑定水鱼成绩 Import-Token。"
            "CLI：`maimai user bind-import --qq <QQ> --token <Import-Token>`；"
            "MCP：`maimai_user_bind_import_token`。"
            "Token 来自水鱼个人资料页，不是开发者 DIVINGFISH_TOKEN。"
        )
    return normalize_token(token)


async def ensure_music_data_cache(*, refresh: bool = False) -> Path:
    from maimai_mcp.resources import music_file

    if music_file.is_file() and not refresh:
        return music_file
    try:
        from maimai_mcp.core.service.diving_fish import get_music_list

        await get_music_list()
    except Exception as exc:  # noqa: BLE001
        if music_file.is_file():
            return music_file
        raise WorkflowError(
            f"无法获取水鱼曲库缓存 ({music_file})：{exc}。"
            "请运行 maimai update music 或检查网络。"
        ) from exc
    if not music_file.is_file():
        raise WorkflowError(
            f"水鱼曲库缓存仍不存在: {music_file}。请运行 maimai update music。"
        )
    return music_file


def make_run_dir(base: Path, qq: str) -> Path:
    stamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    path = base / f"{qq}_{stamp}_{os.getpid()}"
    path.mkdir(parents=True, exist_ok=True)
    return path


def write_raw_export(run_dir: Path, export: dict[str, Any]) -> Path:
    raw_dir = run_dir / "raw"
    raw_dir.mkdir(parents=True, exist_ok=True)
    path = raw_dir / f"sdgb155_full_dump_{datetime.now():%Y%m%d_%H%M%S}.json"
    path.write_text(
        json.dumps(export, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return path


def fetch_official_raw(
    qr_content: str,
    *,
    run_dir: Path,
    keyship: str | None = None,
    place_id: int = DEFAULT_PLACE_ID,
    region_id: int = DEFAULT_REGION_ID,
    timeout: float = 180.0,
    fetch_fn: Callable[[str], dict[str, Any]] | None = None,
) -> Path:
    """SGID → 官服成绩 JSON。fetch_fn 可注入测试用假数据（返回 to_raw_export 形状）。"""
    qr = str(qr_content or "").strip()
    if not qr:
        raise WorkflowError("二维码解析内容不能为空。")

    try:
        if fetch_fn is not None:
            export = fetch_fn(qr)
        else:
            client = MaimaiOfficialClient(
                keychip_id=keyship or DEFAULT_KEYCHIP_ID,
                place_id=place_id,
                region_id=region_id,
                timeout=timeout,
            )
            export = client.fetch_from_sgid(qr).to_raw_export()
    except ChimeSessionError as exc:
        raise WorkflowError(f"获取原始成绩失败：{exc}") from exc
    except OfficialTitleServerError as exc:
        raise WorkflowError(f"获取原始成绩失败：{exc}") from exc
    except OfficialProtocolError as exc:
        raise WorkflowError(f"获取原始成绩失败：{exc}") from exc
    except WorkflowError:
        raise
    except Exception as exc:  # noqa: BLE001
        raise WorkflowError(f"获取原始成绩失败：{exc}") from exc

    if not isinstance(export, dict) or "GetUserMusicApi" not in export:
        raise WorkflowError("获取原始成绩失败：返回数据缺少 GetUserMusicApi。")
    return write_raw_export(run_dir, export)


SOURCE_LABELS = {
    "divingfish": "水鱼",
    "lxns": "落雪",
}


def normalize_sources(sources: Any) -> list[str]:
    """Parse required data source(s): divingfish | lxns | both | list.

    Must be declared explicitly — empty/None is an error.
    """
    if sources is None or (isinstance(sources, str) and not sources.strip()):
        raise WorkflowError(
            "必须声明上传数据源 source。"
            "可选：divingfish（水鱼）/ lxns（落雪）/ both（两者）。"
        )
    if isinstance(sources, (list, tuple, set)):
        items = [str(t).strip().lower() for t in sources if str(t).strip()]
    else:
        text = str(sources).strip().lower()
        if text in ("both", "全部", "all"):
            items = ["divingfish", "lxns"]
        elif "," in text:
            items = [p.strip() for p in text.split(",") if p.strip()]
        else:
            items = [text]
    alias = {
        "df": "divingfish",
        "water": "divingfish",
        "diving-fish": "divingfish",
        "diving_fish": "divingfish",
        "falling": "lxns",
        "snow": "lxns",
        "落雪": "lxns",
        "水鱼": "divingfish",
        "target": "",  # ignore accidental keys
    }
    out: list[str] = []
    for item in items:
        item = alias.get(item, item)
        if not item:
            continue
        if item not in ("divingfish", "lxns"):
            raise WorkflowError(
                f"未知数据源：{item}。必须声明 source 为 "
                "divingfish（水鱼）/ lxns（落雪）/ both（两者）。"
            )
        if item not in out:
            out.append(item)
    if not out:
        raise WorkflowError(
            "必须声明上传数据源 source。"
            "可选：divingfish（水鱼）/ lxns（落雪）/ both（两者）。"
        )
    return out


def format_sources_label(sources: list[str]) -> str:
    return "、".join(SOURCE_LABELS.get(s, s) for s in sources)


def run_convert(
    *,
    raw_json: Path,
    run_dir: Path,
    music_data: Path | None = None,
) -> tuple[Path, Path, list[dict[str, Any]], dict[str, Any]]:
    output_path = run_dir / "update_records.json"
    report_path = run_dir / "update_records_report.json"
    try:
        payload, report = convert_file(
            raw_json,
            output=output_path,
            report=report_path,
            pretty=True,
            music_data=music_data,
        )
    except Exception as exc:  # noqa: BLE001
        raise WorkflowError(f"转换成绩失败：{exc}") from exc
    if not isinstance(payload, list) or not isinstance(report, dict):
        raise WorkflowError("转换输出格式异常。")
    return output_path, report_path, payload, report


def run_convert_lxns(
    *,
    raw_json: Path,
    run_dir: Path,
) -> tuple[Path, Path, list[dict[str, Any]], dict[str, Any]]:
    from .convert_lxns import convert_file_lxns

    output_path = run_dir / "lxns_scores.json"
    report_path = run_dir / "lxns_scores_report.json"
    try:
        payload, report = convert_file_lxns(
            raw_json,
            output=output_path,
            report=report_path,
            pretty=True,
        )
    except Exception as exc:  # noqa: BLE001
        raise WorkflowError(f"转换落雪成绩失败：{exc}") from exc
    if not isinstance(payload, list) or not isinstance(report, dict):
        raise WorkflowError("落雪转换输出格式异常。")
    return output_path, report_path, payload, report


def upload_update_records(
    *,
    import_token: str,
    payload: list[dict[str, Any]],
    api_url: str = DEFAULT_DIVING_FISH_URL,
    timeout: float = 60.0,
    post: Callable[..., Any] = requests.post,
) -> dict[str, Any]:
    response = post(
        api_url,
        headers={
            "Accept": "application/json",
            "Content-Type": "application/json",
            "Import-Token": import_token,
            "User-Agent": "maimai-mcp/0.2.4",
        },
        json=payload,
        timeout=timeout,
    )
    text = str(getattr(response, "text", "") or "")
    status_code = int(getattr(response, "status_code", 0) or 0)
    ok = bool(getattr(response, "ok", False))
    data: Any = None
    try:
        data = response.json()
    except Exception:
        data = text
    if not ok:
        raise WorkflowError(f"上传水鱼失败：HTTP {status_code} {text[:500]}")
    return {"statusCode": status_code, "data": data}


async def upload_lxns_scores(
    *,
    qq: int,
    scores: list[dict[str, Any]],
) -> dict[str, Any]:
    """Upload via Lxns personal OAuth API (requires prior maimai user bind)."""
    from maimai_mcp.core.clients.exceptions import UserNotBindError
    from maimai_mcp.core.clients.lxns.client import LxnsAPI
    from maimai_mcp.core.clients.lxns.exceptions import (
        LXNSOAuthError,
        LXNSParamsError,
        LXNSPermissionDeniedError,
        LXNSTokenError,
        LXNSTooManyRequestsError,
    )
    from maimai_mcp.core.domain.auth import get_token
    from maimai_mcp.core.user import resolve_user

    try:
        user = await resolve_user(qq, auto_create=False)
    except UserNotBindError as exc:
        raise WorkflowError(
            "本地无该 QQ 用户记录。请先 maimai user bind 完成落雪 OAuth 授权。"
        ) from exc

    if not user.access_token and not user.refresh_token:
        raise WorkflowError(
            "尚未绑定落雪 OAuth。请先 maimai user bind / maimai_user_bind_lxns "
            "（授权 scope 需含 write_player）。"
        )
    token = get_token(user)
    api = LxnsAPI(str(qq), token=token)
    try:
        result = await api.upload_scores(scores)
    except (LXNSTokenError, LXNSOAuthError) as exc:
        detail = getattr(exc, "message", None) or str(exc) or "授权失效"
        raise WorkflowError(
            f"落雪授权失效：{detail}。请重新 maimai user bind 完成 OAuth。"
        ) from exc
    except LXNSPermissionDeniedError as exc:
        detail = getattr(exc, "message", None) or str(exc) or "权限不足"
        raise WorkflowError(
            f"落雪拒绝写入：{detail}。请确认 OAuth 含 write_player。"
        ) from exc
    except LXNSParamsError as exc:
        detail = getattr(exc, "message", None) or str(exc) or "参数错误"
        raise WorkflowError(f"上传落雪失败（参数/数据无效）：{detail}") from exc
    except LXNSTooManyRequestsError as exc:
        detail = getattr(exc, "message", None) or str(exc) or "too many requests"
        raise WorkflowError(
            f"上传落雪触发限流：{detail}。请等待 1～2 分钟后再试；"
            "已优化为少请求上传，请确认已重启 MCP。"
        ) from exc
    except Exception as exc:  # noqa: BLE001
        detail = str(exc).strip() or type(exc).__name__
        raise WorkflowError(f"上传落雪失败：{detail}") from exc

    uploaded_n = 0
    if isinstance(result.data, dict):
        uploaded_n = int(result.data.get("uploaded") or 0)
    if not result.success and uploaded_n <= 0:
        raise WorkflowError(
            f"上传落雪失败：code={result.code} {result.message or ''}".strip()
        )
    return {
        "success": result.success or uploaded_n > 0,
        "code": result.code,
        "message": result.message,
        "data": result.data,
        "count": len(scores),
        "uploaded": uploaded_n or len(scores),
    }


async def update_records_workflow(
    *,
    qq: Any,
    qr_content: Any,
    keyship: str | None = None,
    logoutid: int | None = None,
    title_ver: str | None = None,
    timeout: float = 240.0,
    output_dir: Path | None = None,
    refresh_music: bool = False,
    source: Any = None,
    targets: Any = None,
    music_data: Path | None = None,
    fetch_fn: Callable[[str], dict[str, Any]] | None = None,
    post: Callable[..., Any] = requests.post,
) -> dict[str, Any]:
    del logoutid, title_ver  # kept on API for callers; unused after dump merge
    qq_text = normalize_qq(qq)
    qr_text = str(qr_content or "").strip()
    if not qr_text:
        raise WorkflowError("二维码解析内容不能为空。")
    source_list = normalize_sources(source if source is not None else targets)
    need_df = "divingfish" in source_list
    need_lxns = "lxns" in source_list

    import_token: str | None = None
    if need_df:
        import_token = await get_import_token(qq_text)

    music_path: Path | None = None
    if need_df:
        music_path = music_data or await ensure_music_data_cache(refresh=refresh_music)

    run_dir = make_run_dir((output_dir or output_dir_from_env()), qq_text)

    raw_json = fetch_official_raw(
        qr_text,
        run_dir=run_dir,
        keyship=keyship,
        timeout=timeout,
        fetch_fn=fetch_fn,
    )

    uploads: dict[str, Any] = {}
    parts: list[str] = []
    source_label = format_sources_label(source_list)
    result_extra: dict[str, Any] = {
        "rawJsonPath": str(raw_json),
        "source": source_list if len(source_list) > 1 else source_list[0],
        "sources": source_list,
        "source_label": source_label,
        # legacy alias
        "targets": source_list,
    }

    if need_df:
        assert import_token is not None
        payload_path, report_path, payload, report = run_convert(
            raw_json=raw_json,
            run_dir=run_dir,
            music_data=music_path,
        )
        upload = upload_update_records(
            import_token=import_token,
            payload=payload,
            timeout=min(max(timeout / 4, 30), 120),
            post=post,
        )
        uploads["divingfish"] = upload
        result_extra["divingfish"] = {
            "source": "divingfish",
            "source_label": "水鱼",
            "converted": len(payload),
            "skipped": int(report.get("skipped") or 0),
            "payloadPath": str(payload_path),
            "reportPath": str(report_path),
            "musicData": str(music_path) if music_path else None,
            "upload": upload,
        }
        parts.append(
            f"水鱼：转换 {len(payload)} 条，跳过 {int(report.get('skipped') or 0)} 条"
        )

    if need_lxns:
        lx_path, lx_report_path, lx_payload, lx_report = run_convert_lxns(
            raw_json=raw_json,
            run_dir=run_dir,
        )
        lx_upload = await upload_lxns_scores(qq=int(qq_text), scores=lx_payload)
        uploads["lxns"] = lx_upload
        result_extra["lxns"] = {
            "source": "lxns",
            "source_label": "落雪",
            "converted": len(lx_payload),
            "skipped": int(lx_report.get("skipped") or 0),
            "payloadPath": str(lx_path),
            "reportPath": str(lx_report_path),
            "upload": lx_upload,
        }
        uploaded_n = (
            lx_upload.get("uploaded")
            if isinstance(lx_upload.get("uploaded"), int)
            else len(lx_payload)
        )
        fail_n = 0
        if isinstance(lx_upload.get("data"), dict):
            fail_n = int(lx_upload["data"].get("failed_batches") or 0)
        lx_line = (
            f"落雪：转换 {len(lx_payload)} 条，跳过 {int(lx_report.get('skipped') or 0)} 条，"
            f"上传 {uploaded_n} 条"
        )
        if fail_n:
            lx_line += f"，有 {fail_n} 批失败"
        parts.append(lx_line)

    text = f"成绩上传完成（数据源：{source_label}）：" + "；".join(parts) + "。"
    return {
        "ok": True,
        "qq": qq_text,
        "uploads": uploads,
        "text": text,
        **result_extra,
    }
