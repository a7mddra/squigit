# Linux Configuration & Privacy Policy

SnapLLM is designed as a self-contained, XDG-compliant application with native Wayland and X11 support.

## 1. Global Shortcut Configuration

The installer registers `Super+Shift+A` with your desktop environment to trigger a capture.
On Linux, this shortcut does _not_ launch a new binary. Instead, it fires a D-Bus message to the background application's shortcut listener:

```bash
dbus-send --session --type=method_call --dest=com.snapllm.app /com/snapllm/app com.snapllm.app.Capture
```

- **Manual Setup:** If the hotkey does not trigger:
  1. Navigate to **System Settings > Keyboard > Shortcuts**.
  2. Add a **Custom Shortcut**.
  3. **Command:** Input the exact `dbus-send` command above.
  4. **Binding:** Set to `Super+Shift+A`.

## 2. Operational Flow & Portals

Unlike traditional X11 tools, this application respects Wayland security protocols.

- **Trigger:** Press the hotkey to ping the System Tray icon via D-Bus.
- **Capture Strategy:** The capture engine will aggressively attempt a "silent capture" first using CLI fallbacks like `gnome-screenshot`, `grim`, or `spectacle` to provide a seamless UX similar to Flameshot.
- **Portal Fallback:** If the CLI fallbacks fail, a system-level "Screen Share" dialog will appear. This is a mandatory OS security feature. Select your monitor to proceed. Denying this permission will alert you in the UI.

## 3. Application Architecture

- **Single Instance & Autostart:** SnapLLM operates as a single-instance application. It will launch silently to the System Tray on boot if autostart is enabled.
- **Desktop Integration:** An entry is automatically created at `~/.local/share/applications/snapllm.desktop` for launcher access.

## 4. Zero-Trust Architecture

- **Local Storage:** Chats, API keys, and metadata are strictly stored on your local disk using a CAS (Content-Addressable Storage) model.
- **Direct Connection:** API requests are sent directly to the AI provider using your Bring Your Own Key (BYOK) setup.
- **Lens Feature:** This optional feature parses screenshots on third party applications and uses ImgBB as a temporary image host. Do not use "Lens" mode for sensitive personal data.
