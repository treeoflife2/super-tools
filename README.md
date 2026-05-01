
<h1 align="center">Coming Soon</h1>


## Installing

### macOS

Download the `.dmg` for your architecture (Apple Silicon or Intel) from
[Releases](https://github.com/ansxuman/Clauge/releases/latest), open it, and
drag Clauge to your Applications folder.

### Windows

Download the `.exe` installer from
[Releases](https://github.com/ansxuman/Clauge/releases/latest) and run it.

Clauge ships unsigned on Windows (no commercial code-signing certificate).
On first launch you may see a **"Windows protected your PC"** SmartScreen
warning. To proceed:

1. Click **More info**
2. Click **Run anyway**

Each new release retriggers the warning until the binary builds reputation
through downloads. Updates installed via the in-app updater do not retrigger
the warning.

### Linux

Two formats are published per release. Pick whichever your distro prefers:

- **AppImage**: download, `chmod +x Clauge_*.AppImage`, run. Recommended for
  general use — also drives in-app self-update.
- **`.deb`**: `sudo apt install ./Clauge_*.deb` on Debian/Ubuntu derivatives.

Required runtime libs are pulled in automatically by the `.deb` package.
For AppImage on minimal distros, ensure `libwebkit2gtk-4.1-0`, `libgtk-3-0`,
and `libsecret-1-0` are present.


## License

[Business Source License 1.1](LICENSE)
