import zlib, base64, os
import xml.etree.ElementTree as ET
from rich import pretty

local_levels = os.path.join(os.getenv("LOCALAPPDATA"), "GeometryDash", "CCLocalLevels.dat")
decoded_path = "CCLocalLevelsDecoded.xml"

def xor(path, key):
    res = [i ^ key for i in open(path, 'rb').read()]
    return bytearray(res).decode()
 
# decrypt and encrypt functions
def decrypt(data):
    return zlib.decompress(
        base64.b64decode(data.replace('-', '+').replace('_', '/').encode())[10:], -zlib.MAX_WBITS
    )

# find and decrypt local save files and put them into this dict
def get_local_levels():
    print("Decrypting GD File...")
    fin = decrypt(xor(local_levels, 11))

    fw = open(decoded_path, 'wb')
    fw.write(fin)

    return ET.parse(decoded_path)

def format_plist_dict(raw):
    if not raw:
        return {}
    
    keys = raw["k"]
    args = []
    
    if "i" in raw:
        args.extend([int(i) for i in raw["i"]])
    if "s" in raw:
        args.extend(raw["s"])
    if "t" in raw:
        if raw["t"] == None:
            args.extend([False])
        else:
            args.extend([bool(t) for t in raw["t"]])
    if "r" in raw:
        args.extend([float(r) for r in raw["r"]])
    if "d" in raw:
        d = raw["d"]
        if type(d) is list:
            args.extend([format_plist_dict(dic) for dic in d])
        else:
            args.extend([format_plist_dict(raw["d"])])

    return { key: value for key, value in zip(keys, args) }
