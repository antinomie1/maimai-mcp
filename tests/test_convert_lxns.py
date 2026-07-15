from __future__ import annotations

import unittest

from maimai_mcp.core.official.convert_lxns import (
    convert_raw_to_lxns,
    convert_record_lxns,
    official_music_id_to_lxns,
)


class ConvertLxnsTests(unittest.TestCase):
    def test_music_id_mapping(self) -> None:
        self.assertEqual(official_music_id_to_lxns(834), (834, "standard"))
        self.assertEqual(official_music_id_to_lxns(10834), (834, "dx"))
        self.assertEqual(official_music_id_to_lxns(100508), (100508, "utage"))

    def test_convert_record(self) -> None:
        score, skip = convert_record_lxns(
            {
                "musicId": 383,
                "level": 3,
                "achievement": 1010000,
                "deluxscoreMax": 2458,
                "comboStatus": 4,
                "syncStatus": 5,
            }
        )
        self.assertIsNone(skip)
        assert score is not None
        self.assertEqual(score["id"], 383)
        self.assertEqual(score["type"], "standard")
        self.assertEqual(score["level_index"], 3)
        self.assertEqual(score["achievements"], 101.0)
        self.assertEqual(score["fc"], "app")
        self.assertEqual(score["fs"], "sync")
        self.assertEqual(score["dx_score"], 2458)

    def test_skip_zero_and_cap_over_101(self) -> None:
        zero, skip0 = convert_record_lxns(
            {
                "musicId": 17,
                "level": 0,
                "achievement": 0,
                "deluxscoreMax": 0,
                "comboStatus": 0,
                "syncStatus": 0,
            }
        )
        self.assertIsNone(zero)
        self.assertEqual(skip0["reason"], "zero_achievement")
        score, skip = convert_record_lxns(
            {
                "musicId": 100199,
                "level": 0,
                "achievement": 1994804,
                "deluxscoreMax": 100,
                "comboStatus": 0,
                "syncStatus": 5,
            }
        )
        self.assertIsNone(skip)
        assert score is not None
        self.assertEqual(score["achievements"], 101.0)
        self.assertEqual(score["type"], "utage")
        self.assertNotIn("fc", score)
        self.assertEqual(score["fs"], "sync")

    def test_convert_raw_dedup(self) -> None:
        raw = {
            "GetUserMusicApi": {
                "userMusicList": [
                    {
                        "userMusicDetailList": [
                            {
                                "musicId": 11823,
                                "level": 4,
                                "achievement": 1005000,
                                "deluxscoreMax": 100,
                                "comboStatus": 0,
                                "syncStatus": 0,
                            },
                            {
                                "musicId": 11823,
                                "level": 4,
                                "achievement": 1006000,
                                "deluxscoreMax": 200,
                                "comboStatus": 1,
                                "syncStatus": 0,
                            },
                        ]
                    }
                ]
            }
        }
        # Disable catalog filter so unit test does not depend on music_data.json
        payload, skipped = convert_raw_to_lxns(raw, use_catalog=False)
        self.assertEqual(len(payload), 1)
        self.assertEqual(payload[0]["id"], 1823)  # 11823 % 10000
        self.assertEqual(payload[0]["type"], "dx")
        self.assertEqual(skipped[0]["reason"], "duplicate_payload_key")


if __name__ == "__main__":
    unittest.main()
