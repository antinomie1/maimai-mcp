"""Official score dump → convert → Diving-Fish upload tools."""

from __future__ import annotations

from mcp.server.fastmcp import FastMCP

from maimai_mcp.core.errors import ValidationError
from maimai_mcp.core.official.workflow import (
    WorkflowError,
    bind_import_token,
    update_records_workflow,
)
from maimai_mcp.result import FeatureResult

from ..formatters import result_to_json
from ..runtime import ensure_ready, guard_qq, run_fr
from ..schemas import BindImportTokenInput, UpdateRecordsInput


def register(mcp: FastMCP) -> None:
    @mcp.tool(
        name="maimai_user_bind_import_token",
        annotations={
            "title": "Bind Diving-Fish Import-Token",
            "readOnlyHint": False,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    )
    async def maimai_user_bind_import_token(params: BindImportTokenInput) -> str:
        """Bind player QQ to a Diving-Fish score Import-Token (stored in user.db).

        Import-Token is from prober profile → generate import token.
        It is NOT the developer token (DIVINGFISH_TOKEN).
        """

        async def _go():
            await ensure_ready(load_music=False)
            try:
                guard_qq(params.qq)
                result = await bind_import_token(params.qq, params.import_token)
            except ValidationError as e:
                return FeatureResult.failure(e.message, code=e.code)
            except WorkflowError as e:
                return FeatureResult.failure(str(e), code="workflow_error")
            return FeatureResult.success(
                text=result["text"],
                data={
                    "qq": result["qq"],
                    "import_token_bound": True,
                    "import_token_preview": result["import_token_preview"],
                },
            )

        return result_to_json(await run_fr(_go()))

    @mcp.tool(
        name="maimai_update_records",
        annotations={
            "title": "Upload official scores (declare source)",
            "readOnlyHint": False,
            "destructiveHint": False,
            "idempotentHint": False,
            "openWorldHint": True,
        },
    )
    async def maimai_update_records(params: UpdateRecordsInput) -> str:
        """QR login → dump → convert → upload. **source is required.**

        params.source (必填，数据源):
          - divingfish / 水鱼: needs maimai_user_bind_import_token
          - lxns / 落雪: needs maimai_user_bind_lxns (write_player)
          - both: dump once, upload to both

        Do not omit source — never guess the destination.
        """

        async def _go():
            await ensure_ready(load_music=False)
            try:
                guard_qq(params.qq)
                result = await update_records_workflow(
                    qq=params.qq,
                    qr_content=params.qr_content,
                    keyship=params.keyship,
                    logoutid=params.logoutid,
                    title_ver=params.title_ver,
                    timeout=params.timeout,
                    refresh_music=params.refresh_music,
                    source=params.source,
                )
            except ValidationError as e:
                return FeatureResult.failure(e.message, code=e.code)
            except WorkflowError as e:
                return FeatureResult.failure(str(e), code="workflow_error")
            data = {
                "qq": result["qq"],
                "segaUserId": result.get("segaUserId"),
                "source": result.get("source"),
                "sources": result.get("sources"),
                "source_label": result.get("source_label"),
                "rawJsonPath": result.get("rawJsonPath"),
                "uploads": result.get("uploads"),
                "divingfish": result.get("divingfish"),
                "lxns": result.get("lxns"),
            }
            return FeatureResult.success(text=result["text"], data=data)

        return result_to_json(await run_fr(_go()))
