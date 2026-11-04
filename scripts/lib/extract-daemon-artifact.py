"""Extract only CRC-checked, bounded, unambiguous regular CI artifact files."""

import pathlib
import stat
import sys
import unicodedata
import zipfile


def extract(archive: str, destination: str) -> None:
    root = pathlib.Path(destination)
    if root.exists() or root.is_symlink():
        raise ValueError("new extraction directory required")
    with zipfile.ZipFile(archive) as source:
        members = source.infolist()
        if not members or len(members) > 20000 or sum(m.file_size for m in members) > 512 * 1024 * 1024:
            raise ValueError("unsupported artifact size")
        identities = {}
        prefixes = {}
        files = set()
        devices = {"CON", "PRN", "AUX", "NUL"} | {f"{prefix}{n}" for prefix in ("COM", "LPT") for n in range(1, 10)}
        for member in members:
            name = member.orig_filename
            if "\x00" in name or "\\" in name or ":" in name or name.startswith("/"):
                raise ValueError("invalid artifact path")
            parts = name.rstrip("/").split("/")
            if not all(part and part not in (".", "..") for part in parts):
                raise ValueError("ambiguous artifact path")
            for part in parts:
                base = unicodedata.normalize("NFKC", part).split(".")[0].upper()
                if part.endswith((".", " ")) or base in devices or any(ord(c) < 32 or c in '<>"|?*' for c in part):
                    raise ValueError("nonportable Windows artifact component")
            for n in range(1, len(parts) + 1):
                spelling = "/".join(parts[:n])
                identity = unicodedata.normalize("NFC", spelling).casefold()
                if identity in prefixes and prefixes[identity] != spelling:
                    raise ValueError("case-colliding artifact parent")
                prefixes[identity] = spelling
            key = unicodedata.normalize("NFC", "/".join(parts)).casefold()
            if key in identities:
                raise ValueError("duplicate or case-colliding artifact path")
            identities[key] = parts
            mode = stat.S_IFMT(member.external_attr >> 16)
            if mode not in (0, stat.S_IFREG, stat.S_IFDIR):
                raise ValueError("non-regular artifact member")
            if mode and (mode == stat.S_IFDIR) != member.is_dir():
                raise ValueError("contradictory artifact member type")
            if member.flag_bits & 1 or member.file_size > 64 * 1024 * 1024:
                raise ValueError("encrypted or oversized artifact member")
            if not member.is_dir():
                files.add(key)
        for key, parts in identities.items():
            for n in range(1, len(parts)):
                parent = unicodedata.normalize("NFC", "/".join(parts[:n])).casefold()
                if parent in files:
                    raise ValueError("file and directory collide")
        if source.testzip() is not None:
            raise ValueError("artifact CRC mismatch")
        root.mkdir()
        for member in members:
            path = root.joinpath(*member.filename.rstrip("/").split("/"))
            if member.is_dir():
                path.mkdir(parents=True, exist_ok=True)
            else:
                path.parent.mkdir(parents=True, exist_ok=True)
                with source.open(member) as incoming, path.open("xb") as outgoing:
                    while block := incoming.read(1024 * 1024):
                        outgoing.write(block)


if __name__ == "__main__":
    try:
        if len(sys.argv) != 3:
            raise ValueError("archive and new destination required")
        extract(sys.argv[1], sys.argv[2])
    except (ValueError, OSError, zipfile.BadZipFile, RuntimeError):
        print("artifact extraction refused", file=sys.stderr)
        sys.exit(1)
