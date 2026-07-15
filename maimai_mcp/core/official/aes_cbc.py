"""AES-128/192/256 CBC encrypt/decrypt with PKCS7.

Backends (first that loads wins):
1. cryptography (preferred; solid Windows wheels)
2. pycryptodome / Crypto (may break if native .pyd mismatch)
3. pure-Python AES-128 only (always available; dump uses 128-bit keys)
"""

from __future__ import annotations

from typing import Callable

_BACKEND = ""
_encrypt: Callable[[bytes, bytes, bytes], bytes] | None = None
_decrypt: Callable[[bytes, bytes, bytes], bytes] | None = None


def _try_cryptography() -> bool:
    global _BACKEND, _encrypt, _decrypt
    try:
        from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes
    except Exception:
        return False

    def enc(key: bytes, iv: bytes, data: bytes) -> bytes:
        encryptor = Cipher(algorithms.AES(key), modes.CBC(iv)).encryptor()
        return encryptor.update(data) + encryptor.finalize()

    def dec(key: bytes, iv: bytes, data: bytes) -> bytes:
        decryptor = Cipher(algorithms.AES(key), modes.CBC(iv)).decryptor()
        return decryptor.update(data) + decryptor.finalize()

    _BACKEND = "cryptography"
    _encrypt, _decrypt = enc, dec
    return True


def _try_pycryptodome() -> bool:
    global _BACKEND, _encrypt, _decrypt
    try:
        from Crypto.Cipher import AES  # type: ignore
        # Force-load native bits early (import success != usable)
        AES.new(b"\x00" * 16, AES.MODE_CBC, b"\x00" * 16)
    except Exception:
        return False

    def enc(key: bytes, iv: bytes, data: bytes) -> bytes:
        return AES.new(key, AES.MODE_CBC, iv).encrypt(data)

    def dec(key: bytes, iv: bytes, data: bytes) -> bytes:
        return AES.new(key, AES.MODE_CBC, iv).decrypt(data)

    _BACKEND = "pycryptodome"
    _encrypt, _decrypt = enc, dec
    return True


# --- pure-Python AES-128/192/256 (Rijndael) ---

_SBOX = bytes(
    [
        0x63, 0x7C, 0x77, 0x7B, 0xF2, 0x6B, 0x6F, 0xC5, 0x30, 0x01, 0x67, 0x2B, 0xFE, 0xD7, 0xAB, 0x76,
        0xCA, 0x82, 0xC9, 0x7D, 0xFA, 0x59, 0x47, 0xF0, 0xAD, 0xD4, 0xA2, 0xAF, 0x9C, 0xA4, 0x72, 0xC0,
        0xB7, 0xFD, 0x93, 0x26, 0x36, 0x3F, 0xF7, 0xCC, 0x34, 0xA5, 0xE5, 0xF1, 0x71, 0xD8, 0x31, 0x15,
        0x04, 0xC7, 0x23, 0xC3, 0x18, 0x96, 0x05, 0x9A, 0x07, 0x12, 0x80, 0xE2, 0xEB, 0x27, 0xB2, 0x75,
        0x09, 0x83, 0x2C, 0x1A, 0x1B, 0x6E, 0x5A, 0xA0, 0x52, 0x3B, 0xD6, 0xB3, 0x29, 0xE3, 0x2F, 0x84,
        0x53, 0xD1, 0x00, 0xED, 0x20, 0xFC, 0xB1, 0x5B, 0x6A, 0xCB, 0xBE, 0x39, 0x4A, 0x4C, 0x58, 0xCF,
        0xD0, 0xEF, 0xAA, 0xFB, 0x43, 0x4D, 0x33, 0x85, 0x45, 0xF9, 0x02, 0x7F, 0x50, 0x3C, 0x9F, 0xA8,
        0x51, 0xA3, 0x40, 0x8F, 0x92, 0x9D, 0x38, 0xF5, 0xBC, 0xB6, 0xDA, 0x21, 0x10, 0xFF, 0xF3, 0xD2,
        0xCD, 0x0C, 0x13, 0xEC, 0x5F, 0x97, 0x44, 0x17, 0xC4, 0xA7, 0x7E, 0x3D, 0x64, 0x5D, 0x19, 0x73,
        0x60, 0x81, 0x4F, 0xDC, 0x22, 0x2A, 0x90, 0x88, 0x46, 0xEE, 0xB8, 0x14, 0xDE, 0x5E, 0x0B, 0xDB,
        0xE0, 0x32, 0x3A, 0x0A, 0x49, 0x06, 0x24, 0x5C, 0xC2, 0xD3, 0xAC, 0x62, 0x91, 0x95, 0xE4, 0x79,
        0xE7, 0xC8, 0x37, 0x6D, 0x8D, 0xD5, 0x4E, 0xA9, 0x6C, 0x56, 0xF4, 0xEA, 0x65, 0x7A, 0xAE, 0x08,
        0xBA, 0x78, 0x25, 0x2E, 0x1C, 0xA6, 0xB4, 0xC6, 0xE8, 0xDD, 0x74, 0x1F, 0x4B, 0xBD, 0x8B, 0x8A,
        0x70, 0x3E, 0xB5, 0x66, 0x48, 0x03, 0xF6, 0x0E, 0x61, 0x35, 0x57, 0xB9, 0x86, 0xC1, 0x1D, 0x9E,
        0xE1, 0xF8, 0x98, 0x11, 0x69, 0xD9, 0x8E, 0x94, 0x9B, 0x1E, 0x87, 0xE9, 0xCE, 0x55, 0x28, 0xDF,
        0x8C, 0xA1, 0x89, 0x0D, 0xBF, 0xE6, 0x42, 0x68, 0x41, 0x99, 0x2D, 0x0F, 0xB0, 0x54, 0xBB, 0x16,
    ]
)
_INV_SBOX = bytes([_SBOX.index(i) for i in range(256)])
_RCON = (0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1B, 0x36)


def _mul(a: int, b: int) -> int:
    p = 0
    for _ in range(8):
        if b & 1:
            p ^= a
        hi = a & 0x80
        a = (a << 1) & 0xFF
        if hi:
            a ^= 0x1B
        b >>= 1
    return p


def _expand_key(key: bytes) -> list[list[int]]:
    key_len = len(key)
    if key_len == 16:
        nk, nr = 4, 10
    elif key_len == 24:
        nk, nr = 6, 12
    elif key_len == 32:
        nk, nr = 8, 14
    else:
        raise ValueError(f"unsupported AES key length: {key_len}")
    total = 16 * (nr + 1)
    w = list(key)
    i = 1
    while len(w) < total:
        t = w[-4:]
        pos = len(w)
        if pos % (4 * nk) == 0:
            t = [t[1], t[2], t[3], t[0]]
            t = [_SBOX[b] for b in t]
            t[0] ^= _RCON[i]
            i += 1
        elif nk > 6 and pos % (4 * nk) == 16:
            t = [_SBOX[b] for b in t]
        for j in range(4):
            w.append(w[pos - 4 * nk + j] ^ t[j])
    return [list(w[r * 16 : (r + 1) * 16]) for r in range(nr + 1)]


def _add_round_key(state: list[int], rk: list[int]) -> None:
    for i in range(16):
        state[i] ^= rk[i]


def _sub_bytes(state: list[int], box: bytes) -> None:
    for i in range(16):
        state[i] = box[state[i]]


def _shift_rows(state: list[int]) -> None:
    s = state
    s[1], s[5], s[9], s[13] = s[5], s[9], s[13], s[1]
    s[2], s[6], s[10], s[14] = s[10], s[14], s[2], s[6]
    s[3], s[7], s[11], s[15] = s[15], s[3], s[7], s[11]


def _inv_shift_rows(state: list[int]) -> None:
    s = state
    s[1], s[5], s[9], s[13] = s[13], s[1], s[5], s[9]
    s[2], s[6], s[10], s[14] = s[10], s[14], s[2], s[6]
    s[3], s[7], s[11], s[15] = s[7], s[11], s[15], s[3]


def _mix_columns(state: list[int]) -> None:
    for c in range(4):
        i = c * 4
        a0, a1, a2, a3 = state[i : i + 4]
        state[i] = _mul(a0, 2) ^ _mul(a1, 3) ^ a2 ^ a3
        state[i + 1] = a0 ^ _mul(a1, 2) ^ _mul(a2, 3) ^ a3
        state[i + 2] = a0 ^ a1 ^ _mul(a2, 2) ^ _mul(a3, 3)
        state[i + 3] = _mul(a0, 3) ^ a1 ^ a2 ^ _mul(a3, 2)


def _inv_mix_columns(state: list[int]) -> None:
    for c in range(4):
        i = c * 4
        a0, a1, a2, a3 = state[i : i + 4]
        state[i] = _mul(a0, 14) ^ _mul(a1, 11) ^ _mul(a2, 13) ^ _mul(a3, 9)
        state[i + 1] = _mul(a0, 9) ^ _mul(a1, 14) ^ _mul(a2, 11) ^ _mul(a3, 13)
        state[i + 2] = _mul(a0, 13) ^ _mul(a1, 9) ^ _mul(a2, 14) ^ _mul(a3, 11)
        state[i + 3] = _mul(a0, 11) ^ _mul(a1, 13) ^ _mul(a2, 9) ^ _mul(a3, 14)


def _encrypt_block(block: bytes, round_keys: list[list[int]]) -> bytes:
    state = list(block)
    nr = len(round_keys) - 1
    _add_round_key(state, round_keys[0])
    for r in range(1, nr):
        _sub_bytes(state, _SBOX)
        _shift_rows(state)
        _mix_columns(state)
        _add_round_key(state, round_keys[r])
    _sub_bytes(state, _SBOX)
    _shift_rows(state)
    _add_round_key(state, round_keys[nr])
    return bytes(state)


def _decrypt_block(block: bytes, round_keys: list[list[int]]) -> bytes:
    state = list(block)
    nr = len(round_keys) - 1
    _add_round_key(state, round_keys[nr])
    for r in range(nr - 1, 0, -1):
        _inv_shift_rows(state)
        _sub_bytes(state, _INV_SBOX)
        _add_round_key(state, round_keys[r])
        _inv_mix_columns(state)
    _inv_shift_rows(state)
    _sub_bytes(state, _INV_SBOX)
    _add_round_key(state, round_keys[0])
    return bytes(state)


def _pure_encrypt(key: bytes, iv: bytes, data: bytes) -> bytes:
    rk = _expand_key(key)
    out = bytearray()
    prev = iv
    for i in range(0, len(data), 16):
        block = bytes(a ^ b for a, b in zip(data[i : i + 16], prev))
        enc = _encrypt_block(block, rk)
        out.extend(enc)
        prev = enc
    return bytes(out)


def _pure_decrypt(key: bytes, iv: bytes, data: bytes) -> bytes:
    rk = _expand_key(key)
    out = bytearray()
    prev = iv
    for i in range(0, len(data), 16):
        block = data[i : i + 16]
        dec = _decrypt_block(block, rk)
        out.extend(a ^ b for a, b in zip(dec, prev))
        prev = block
    return bytes(out)


def _use_pure() -> None:
    global _BACKEND, _encrypt, _decrypt
    _BACKEND = "pure-python"
    _encrypt, _decrypt = _pure_encrypt, _pure_decrypt


def _ensure_backend() -> None:
    global _encrypt, _decrypt
    if _encrypt is not None and _decrypt is not None:
        return
    if _try_cryptography():
        return
    if _try_pycryptodome():
        return
    _use_pure()


def aes_backend() -> str:
    _ensure_backend()
    return _BACKEND


def aes_cbc_encrypt(key: bytes, iv: bytes, plaintext_padded: bytes) -> bytes:
    """Encrypt already-PKCS7-padded plaintext with AES-CBC."""
    if len(key) not in (16, 24, 32):
        raise ValueError(f"invalid AES key length: {len(key)}")
    if len(iv) != 16:
        raise ValueError(f"invalid AES IV length: {len(iv)}")
    if len(plaintext_padded) % 16 != 0:
        raise ValueError("ciphertext/plaintext length must be multiple of 16")
    _ensure_backend()
    assert _encrypt is not None
    return _encrypt(key, iv, plaintext_padded)


def aes_cbc_decrypt(key: bytes, iv: bytes, ciphertext: bytes) -> bytes:
    """Decrypt AES-CBC ciphertext; returns PKCS7-padded plaintext."""
    if len(key) not in (16, 24, 32):
        raise ValueError(f"invalid AES key length: {len(key)}")
    if len(iv) != 16:
        raise ValueError(f"invalid AES IV length: {len(iv)}")
    if len(ciphertext) % 16 != 0 or not ciphertext:
        raise ValueError("ciphertext length must be positive multiple of 16")
    _ensure_backend()
    assert _decrypt is not None
    return _decrypt(key, iv, ciphertext)
