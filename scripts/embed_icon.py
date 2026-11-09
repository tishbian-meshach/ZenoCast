"""Embeds assets/icon.ico into a target Windows PE executable (.exe).

Uses standard Win32 UpdateResourceW via ctypes (kernel32.dll), requiring no
external dependencies or MSVC SDK tools.
"""

import os
import struct
import sys
import ctypes
from ctypes import wintypes


def embed_icon(exe_path: str, ico_path: str) -> bool:
    if not os.path.exists(exe_path):
        print(f"Target executable does not exist: {exe_path}")
        return False
    if not os.path.exists(ico_path):
        print(f"Icon file does not exist: {ico_path}")
        return False

    with open(ico_path, "rb") as f:
        ico_data = f.read()

    reserved, ico_type, count = struct.unpack_from("<HHH", ico_data, 0)
    grp_header = struct.pack("<HHH", reserved, ico_type, count)
    entries = []
    images = []
    offset = 6
    for i in range(count):
        (
            bWidth,
            bHeight,
            bColorCount,
            bReserved,
            wPlanes,
            wBitCount,
            dwBytesInRes,
            dwImageOffset,
        ) = struct.unpack_from("<BBBBHHII", ico_data, offset)
        offset += 16
        img_bytes = ico_data[dwImageOffset : dwImageOffset + dwBytesInRes]
        images.append((i + 1, img_bytes))
        entry = struct.pack(
            "<BBBBHHIH",
            bWidth,
            bHeight,
            bColorCount,
            bReserved,
            wPlanes,
            wBitCount,
            dwBytesInRes,
            i + 1,
        )
        entries.append(entry)

    grp_icon_data = grp_header + b"".join(entries)

    kernel32 = ctypes.windll.kernel32
    kernel32.BeginUpdateResourceW.restype = wintypes.HANDLE
    kernel32.BeginUpdateResourceW.argtypes = [wintypes.LPCWSTR, wintypes.BOOL]

    kernel32.UpdateResourceW.restype = wintypes.BOOL
    kernel32.UpdateResourceW.argtypes = [
        wintypes.HANDLE,
        ctypes.c_void_p,
        ctypes.c_void_p,
        wintypes.WORD,
        ctypes.c_void_p,
        wintypes.DWORD,
    ]

    kernel32.EndUpdateResourceW.restype = wintypes.BOOL
    kernel32.EndUpdateResourceW.argtypes = [wintypes.HANDLE, wintypes.BOOL]

    RT_ICON = 3
    RT_GROUP_ICON = 14

    h_update = kernel32.BeginUpdateResourceW(exe_path, False)
    if not h_update:
        print("BeginUpdateResourceW failed:", kernel32.GetLastError())
        return False

    # Write RT_GROUP_ICON (ID 1)
    if not kernel32.UpdateResourceW(
        h_update, RT_GROUP_ICON, 1, 0, grp_icon_data, len(grp_icon_data)
    ):
        print("Update RT_GROUP_ICON failed:", kernel32.GetLastError())
        kernel32.EndUpdateResourceW(h_update, True)
        return False

    # Write RT_ICON for each mipmap image
    for icon_id, img_data in images:
        if not kernel32.UpdateResourceW(
            h_update, RT_ICON, icon_id, 0, img_data, len(img_data)
        ):
            print(f"Update RT_ICON {icon_id} failed:", kernel32.GetLastError())
            kernel32.EndUpdateResourceW(h_update, True)
            return False

    res = kernel32.EndUpdateResourceW(h_update, False)
    if not res:
        print("EndUpdateResourceW failed:", kernel32.GetLastError())
        return False

    print(f"Successfully embedded icon into {exe_path}")
    return True


if __name__ == "__main__":
    target = sys.argv[1] if len(sys.argv) > 1 else "target/debug/zenocast.exe"
    icon = sys.argv[2] if len(sys.argv) > 2 else "assets/icon.ico"
    success = embed_icon(target, icon)
    sys.exit(0 if success else 1)
