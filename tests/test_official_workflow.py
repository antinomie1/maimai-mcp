from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from typing import Any
from unittest.mock import AsyncMock, patch

from maimai_mcp.core.official import workflow


class FakeResponse:
    ok = True
    status_code = 200
    text = '{"updated":true}'

    def json(self) -> dict[str, Any]:
        return {"updated": True}


class OfficialWorkflowTests(unittest.IsolatedAsyncioTestCase):
    async def test_bind_import_token_masks_secret(self) -> None:
        with patch(
            "maimai_mcp.core.database.qq.update_user",
            new_callable=AsyncMock,
        ) as mock_update:
            mock_update.return_value = None
            result = await workflow.bind_import_token(
                "1000000001", "import-token-abcdef"
            )
        mock_update.assert_awaited_once()
        kwargs = mock_update.await_args.kwargs
        self.assertEqual(kwargs["import_token"], "import-token-abcdef")
        self.assertTrue(result["ok"])
        self.assertNotIn("import-token-abcdef", result["text"])
        self.assertTrue(result["import_token_bound"])

    async def test_update_workflow_fetch_convert_upload(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            music_data = root / "music_data.json"
            music_data.write_text(
                json.dumps([{"id": 383, "title": "Link(CoF)", "type": "SD"}]),
                encoding="utf-8",
            )
            output_dir = root / "runs"

            def fake_fetch(_qr: str) -> dict[str, Any]:
                return {
                    "GetUserMusicApi": {
                        "userMusicList": [
                            {
                                "userMusicDetailList": [
                                    {
                                        "musicId": 383,
                                        "level": 3,
                                        "achievement": 1000000,
                                        "deluxscoreMax": 2458,
                                        "comboStatus": 1,
                                        "syncStatus": 5,
                                    }
                                ]
                            }
                        ]
                    }
                }

            with (
                patch.object(
                    workflow,
                    "get_import_token",
                    new_callable=AsyncMock,
                    return_value="import-token-abcdef",
                ),
                patch.object(
                    workflow,
                    "ensure_music_data_cache",
                    new_callable=AsyncMock,
                    return_value=music_data,
                ),
            ):
                result = await workflow.update_records_workflow(
                    qq="1000000001",
                    qr_content="SGWCFAKE",
                    source="divingfish",
                    output_dir=output_dir,
                    music_data=music_data,
                    fetch_fn=fake_fetch,
                    post=lambda *a, **k: FakeResponse(),
                )
                self.assertTrue(Path(result["rawJsonPath"]).is_file())

        self.assertTrue(result["ok"])
        self.assertEqual(result["source"], "divingfish")
        self.assertEqual(result["sources"], ["divingfish"])
        self.assertEqual(result["source_label"], "水鱼")
        self.assertEqual(result["divingfish"]["converted"], 1)
        self.assertEqual(result["divingfish"]["skipped"], 0)
        self.assertNotIn("segaUserId", result)
        self.assertIn("数据源：水鱼", result["text"])
        self.assertNotIn("import-token-abcdef", result["text"])
        self.assertNotIn("SEGA userId", result["text"])

    def test_source_required(self) -> None:
        with self.assertRaises(workflow.WorkflowError) as ctx:
            workflow.normalize_sources(None)
        self.assertIn("必须声明", str(ctx.exception))


if __name__ == "__main__":
    unittest.main()
