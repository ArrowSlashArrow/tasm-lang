from deserialiser import local_levels
import os
backup_path = os.path.join(os.getenv("LOCALAPPDATA"), "GeometryDash", "OLDCCLOCALLEVELS.dat")
open(local_levels, "wb").write(open(backup_path, "rb").read())