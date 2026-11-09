"""Installs a Start Menu shortcut for Zenocast Studio with explicit AppUserModelID.

Windows 10/11 taskbar groupings use the Start Menu shortcut and its AppUserModelID
to resolve and display the application's icon on the taskbar.
"""

import os
import sys
import ctypes

try:
    from win32com.propsys import propsys
    import win32com.client
except ImportError:
    pass


def install_shortcut():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    exe_path = os.path.join(repo_root, "target", "debug", "zenocast.exe")
    ico_path = os.path.join(repo_root, "assets", "icon.ico")
    app_id = "Zenocast.Studio.App"

    programs_dir = os.path.expandvars(
        r"%APPDATA%\Microsoft\Windows\Start Menu\Programs"
    )
    lnk_path = os.path.join(programs_dir, "Zenocast Studio.lnk")

    # 1. Create or update shell link via WScript.Shell
    import win32com.client

    wsh = win32com.client.Dispatch("WScript.Shell")
    shortcut = wsh.CreateShortcut(lnk_path)
    shortcut.TargetPath = exe_path
    shortcut.WorkingDirectory = repo_root
    shortcut.IconLocation = f"{ico_path},0"
    shortcut.Description = "Zenocast Studio"
    shortcut.Save()

    # 2. Set System.AppUserModel.ID property
    try:
        from win32com.propsys import propsys

        store = propsys.SHGetPropertyStoreFromParsingName(
            lnk_path, None, 2, propsys.IID_IPropertyStore
        )
        pkey = propsys.PSGetPropertyKeyFromName("System.AppUserModel.ID")
        prop_val = propsys.PROPVARIANTType(app_id)
        store.SetValue(pkey, prop_val)
        store.Commit()
    except Exception as e:
        print("Note: Failed to set property store:", e)

    # 3. Notify Windows Shell
    shell32 = ctypes.windll.shell32
    SHCNE_ASSOCCHANGED = 0x08000000
    SHCNF_IDLIST = 0
    shell32.SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None)

    print(f"Successfully installed Zenocast Studio shortcut at: {lnk_path}")


if __name__ == "__main__":
    install_shortcut()
