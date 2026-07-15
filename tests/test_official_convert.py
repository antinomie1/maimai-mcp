from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from maimai_mcp.core.official import convert as converter


class ConvertOfficialRawRecordsTests(unittest.TestCase):
    def test_convert_records_uses_divingfish_music_data_titles(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            music_data = root / "music_data.json"
            music_data.write_text(
                json.dumps(
                    [
                        {"id": "383", "title": "Link(CoF)", "type": "SD"},
                        {"id": "11823", "title": "Zitronectar", "type": "DX"},
                        {"id": "100508", "title": "[協]恋愛裁判", "type": "DX"},
                    ],
                    ensure_ascii=False,
                ),
                encoding="utf-8",
            )
            music_index = converter.build_music_index(music_data=music_data)
            raw = {
                "GetUserMusicApi": {
                    "userMusicList": [
                        {
                            "userMusicDetailList": [
                                {
                                    "musicId": 383,
                                    "level": 3,
                                    "achievement": 1012345,
                                    "deluxscoreMax": 2458,
                                    "comboStatus": 2,
                                    "syncStatus": 5,
                                },
                                {
                                    "musicId": 11823,
                                    "level": 4,
                                    "achievement": 1007770,
                                    "deluxscoreMax": "2711",
                                    "comboStatus": 4,
                                    "syncStatus": 4,
                                },
                                {
                                    "musicId": 999999,
                                    "level": 3,
                                    "achievement": 1000000,
                                    "deluxscoreMax": 1000,
                                    "comboStatus": 0,
                                    "syncStatus": 0,
                                },
                            ]
                        }
                    ]
                }
            }
            payload, skipped = converter.convert_raw_records(
                raw, music_index=music_index
            )

        self.assertEqual(len(payload), 2)
        self.assertEqual(payload[0]["title"], "Link(CoF)")
        self.assertEqual(payload[0]["type"], "SD")
        self.assertEqual(payload[0]["achievements"], 101.2345)
        self.assertEqual(payload[0]["fc"], "fcp")
        self.assertEqual(payload[0]["fs"], "sync")
        self.assertEqual(payload[1]["title"], "Zitronectar")
        self.assertEqual(payload[1]["fs"], "fsdp")
        self.assertEqual(skipped[0]["reason"], "music_id_not_found")

    def test_convert_file_writes_payload_and_report(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            music_data = root / "music_data.json"
            music_data.write_text(
                json.dumps([{"id": 383, "title": "Link(CoF)", "type": "SD"}]),
                encoding="utf-8",
            )
            raw_path = root / "raw.json"
            raw_path.write_text(
                json.dumps(
                    {
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
                ),
                encoding="utf-8",
            )
            out = root / "out.json"
            report = root / "report.json"
            payload, body = converter.convert_file(
                raw_path,
                output=out,
                report=report,
                pretty=True,
                music_data=music_data,
            )

            self.assertEqual(len(payload), 1)
            self.assertEqual(body["converted"], 1)
            self.assertEqual(body["musicData"], str(music_data))
            self.assertTrue(out.is_file())
            self.assertTrue(report.is_file())


if __name__ == "__main__":
    unittest.main()
