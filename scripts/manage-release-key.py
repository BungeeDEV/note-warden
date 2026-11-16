"""Initialize the Minisign release key without installing executables locally.

Uses PyCA Ed25519 and Minisign's documented key serialization. Only the public
key and a signed test message enter the source tree. The local private backup
is encrypted with Windows DPAPI; GitHub receives it only as an Actions secret.
"""

import argparse
import base64
import ctypes
from ctypes import wintypes
import hashlib
import os
from pathlib import Path
import re
import subprocess
import time

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey


ROOT = Path(__file__).resolve().parent.parent


class Blob(ctypes.Structure):
    _fields_ = [("length", wintypes.DWORD), ("data", ctypes.POINTER(ctypes.c_ubyte))]


def dpapi(data, decrypt=False):
    if os.name != "nt":
        raise RuntimeError("The local signing-key backup requires Windows DPAPI")
    source_buffer = ctypes.create_string_buffer(data)
    source = Blob(len(data), ctypes.cast(source_buffer, ctypes.POINTER(ctypes.c_ubyte)))
    output = Blob()
    crypt32 = ctypes.WinDLL("crypt32", use_last_error=True)
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    if decrypt:
        function = crypt32.CryptUnprotectData
        function.argtypes = [ctypes.POINTER(Blob), ctypes.c_void_p, ctypes.POINTER(Blob),
                             ctypes.c_void_p, ctypes.c_void_p, wintypes.DWORD, ctypes.POINTER(Blob)]
        description = None
    else:
        function = crypt32.CryptProtectData
        function.argtypes = [ctypes.POINTER(Blob), wintypes.LPCWSTR, ctypes.POINTER(Blob),
                             ctypes.c_void_p, ctypes.c_void_p, wintypes.DWORD, ctypes.POINTER(Blob)]
        description = "Note Warden release signing key"
    function.restype = wintypes.BOOL
    kernel32.LocalFree.argtypes = [ctypes.c_void_p]
    kernel32.LocalFree.restype = ctypes.c_void_p
    if not function(ctypes.byref(source), description, None, None, None, 1, ctypes.byref(output)):
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        return ctypes.string_at(output.data, output.length)
    finally:
        kernel32.LocalFree(output.data)


def new_secret():
    private = Ed25519PrivateKey.generate()
    seed = private.private_bytes(serialization.Encoding.Raw, serialization.PrivateFormat.Raw,
                                 serialization.NoEncryption())
    public = private.public_key().public_bytes(serialization.Encoding.Raw, serialization.PublicFormat.Raw)
    key_id = os.urandom(8)
    sodium_secret = seed + public
    checksum = hashlib.blake2b(b"Ed" + key_id + sodium_secret, digest_size=32).digest()
    # Minisign SeckeyStruct: Ed, KDFNONE, B2, zero KDF fields, key id, seed+pub, checksum.
    return b"Ed\0\0B2" + bytes(48) + key_id + sodium_secret + checksum


def validate_secret(secret):
    if len(secret) != 158 or secret[:6] != b"Ed\0\0B2":
        raise RuntimeError("Unexpected release-key format")
    key_id, sodium_secret, checksum = secret[54:62], secret[62:126], secret[126:158]
    if hashlib.blake2b(b"Ed" + key_id + sodium_secret, digest_size=32).digest() != checksum:
        raise RuntimeError("Release-key checksum mismatch")
    private = Ed25519PrivateKey.from_private_bytes(sodium_secret[:32])
    public = private.public_key().public_bytes(serialization.Encoding.Raw, serialization.PublicFormat.Raw)
    if public != sodium_secret[32:]:
        raise RuntimeError("Release public/private key mismatch")
    return private, public, key_id


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--upload", action="store_true", help="Store the key as an encrypted GitHub Actions secret")
    parser.add_argument("--environment", default="release", help="GitHub environment for the signing secret (default: release)")
    parser.add_argument("--export", type=Path, help="Explicitly export an unencrypted Minisign key outside the project")
    args = parser.parse_args()
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", args.repository):
        raise RuntimeError("Invalid repository name")
    if not re.fullmatch(r"[A-Za-z0-9_-]+", args.environment):
        raise RuntimeError("Invalid environment name")
    key_dir = Path(os.environ["LOCALAPPDATA"]) / "NoteWardenSigning"
    key_dir.mkdir(parents=True, exist_ok=True)
    backup = key_dir / "note-warden.key.dpapi"
    public_file = ROOT / "installer/update-signing.pub"
    if backup.exists():
        secret = dpapi(backup.read_bytes(), decrypt=True)
    else:
        if public_file.exists():
            raise RuntimeError("Public key exists without its private backup; refusing to rotate it")
        secret = new_secret()
        with backup.open("xb") as file:
            file.write(dpapi(secret))
    private, public, key_id = validate_secret(secret)
    encoded_public = base64.b64encode(b"Ed" + key_id + public).decode("ascii")
    public_text = "untrusted comment: Note Warden update signing public key\n" + encoded_public + "\n"
    if public_file.exists() and public_file.read_text(encoding="utf8") != public_text:
        raise RuntimeError("Existing public key differs from the protected backup; refusing rotation")
    public_file.write_text(public_text, encoding="utf8")
    secret_text = ("untrusted comment: minisign secret key\n" + base64.b64encode(secret).decode("ascii") + "\n").encode("ascii")
    if args.export:
        target = args.export.resolve()
        if target.is_relative_to(ROOT):
            raise RuntimeError("Private keys must remain outside the project")
        with target.open("xb") as file:
            file.write(secret_text)
        print("Private key exported to the explicitly selected external file; protect that file.")
    if args.upload:
        result = subprocess.run(["gh", "secret", "set", "NOTE_WARDEN_MINISIGN_KEY", "--repo", args.repository,
                                 "--env", args.environment],
                                input=secret_text, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
        if result.returncode:
            raise RuntimeError("GitHub secret upload failed; the protected local key was retained")
        print("GitHub Actions signing secret stored successfully.")
    fixture = b"Note Warden release signing key verification.\n"
    signature = private.sign(hashlib.blake2b(fixture).digest())
    trusted = f"timestamp:{int(time.time())}\tfile:release-key-check.txt"
    global_signature = private.sign(signature + trusted.encode("utf8"))
    fixture_signature = ("untrusted comment: Note Warden release key interoperability test\n"
                         + base64.b64encode(b"ED" + key_id + signature).decode("ascii")
                         + "\ntrusted comment: " + trusted + "\n"
                         + base64.b64encode(global_signature).decode("ascii") + "\n")
    (ROOT / "installer/release-key-check.txt").write_bytes(fixture)
    (ROOT / "installer/release-key-check.txt.minisig").write_text(fixture_signature, encoding="utf8")
    print("Public key and signed interoperability fixture are ready.")
    print("Private backup protected with DPAPI outside the project:", backup)


if __name__ == "__main__":
    main()
