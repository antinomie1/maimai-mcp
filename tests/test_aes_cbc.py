from __future__ import annotations

import unittest

from maimai_mcp.core.official import aes_cbc
from maimai_mcp.core.official.aes_cbc import aes_backend, aes_cbc_decrypt, aes_cbc_encrypt
from maimai_mcp.core.official.protocol import AES_IV as GAME_AES_IV
from maimai_mcp.core.official.protocol import AES_KEY as GAME_AES_KEY


class AesCbcTests(unittest.TestCase):
    def test_roundtrip_preferred_backend(self) -> None:
        key = b"0123456789abcdef"
        iv = b"fedcba9876543210"
        plain = b"hello official dump!!"
        pad = 16 - (len(plain) % 16)
        padded = plain + bytes([pad]) * pad
        ct = aes_cbc_encrypt(key, iv, padded)
        pt = aes_cbc_decrypt(key, iv, ct)
        self.assertEqual(pt, padded)
        self.assertIn(aes_backend(), {"cryptography", "pycryptodome", "pure-python"})

    def test_game_key_aes256_pure_matches_preferred(self) -> None:
        self.assertEqual(len(GAME_AES_KEY), 32)
        self.assertEqual(len(GAME_AES_IV), 16)
        padded = b"\x10" * 16
        ct_pref = aes_cbc_encrypt(GAME_AES_KEY, GAME_AES_IV, padded)
        pt_pref = aes_cbc_decrypt(GAME_AES_KEY, GAME_AES_IV, ct_pref)
        self.assertEqual(pt_pref, padded)
        ct_pure = aes_cbc._pure_encrypt(GAME_AES_KEY, GAME_AES_IV, padded)
        self.assertEqual(ct_pure, ct_pref)
        self.assertEqual(
            aes_cbc._pure_decrypt(GAME_AES_KEY, GAME_AES_IV, ct_pure), padded
        )


if __name__ == "__main__":
    unittest.main()
