import gzip, base64, zlib
import xml.etree.ElementTree as ET

def dict_to_etree(d):
    assert isinstance(d, dict) and len(d) == 1
    tag, content = next(iter(d.items()))
    root = ET.Element(tag)
    build_tree(root, content)
    return root

def build_tree(elem, content):
    if isinstance(content, dict):
        for k, v in content.items():
            child = ET.SubElement(elem, k)
            build_tree(child, v)
    elif isinstance(content, list):
        for v in content:
            child = ET.SubElement(elem, elem.tag)
            build_tree(child, v)
    elif content is not None:
        elem.text = str(content)


def encrypt_level_string(s):
    raw = b"H4sIAAAAAA" + zlib.compress(s.encode())[2:-4]
    crc32 = zlib.crc32(s.encode()).to_bytes(4, byteorder="little")
    datasize = len(s).to_bytes(4, byteorder="little")
    return b"H4sIAAAAAAAAC" + base64.b64encode(
        raw + crc32 + datasize
    ).replace(b"+", b"-").replace(b"/", b"_")[13:]  # 14th char is apparently a checksum value or something. do not touch it.


def encrypt_savefile_str(s):
    compressed = zlib.compress(s)

    gzip_signature = b"\x1f\x8b\x08\x00\x00\x00\x00\x00\x00\x0b"
    deflate = compressed[2:-4]
    crc32 = zlib.crc32(s).to_bytes(4, byteorder="little")
    datasize = len(s).to_bytes(4, byteorder="little")

    combined = gzip_signature + deflate + crc32 + datasize
    base64ed = base64.b64encode(combined)
    swapped_chars = base64ed.replace(b"+", b"-").replace(b"/", b"_")
    return bytes([char ^ 11 for char in swapped_chars])
