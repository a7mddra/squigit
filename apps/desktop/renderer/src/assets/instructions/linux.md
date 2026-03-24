# We're thrilled to have you!

Before you dive in, let's quickly go over how Squigit integrates with your system, how to get started, and how we protect your data.

## 1. Global Shortcuts & System Tray

We've attempted to register `Super+Shift+A` with your desktop environment so you can trigger a capture from anywhere. Go ahead and press it to test it out!

**Important:** For this to work instantly, Squigit needs to stay running in your system tray. It's incredibly lightweight, so you can safely leave it alive 24/7 to guarantee a rapid response when you need it.

<details>
<summary><strong>Not working? Expand for details</strong></summary>

Because Linux environments vary wildly, automated installation sometimes fails on specific setups (especially Wayland). If the shortcut didn't work, we recommend manually copying this command:

```bash
/bin/sh -lc 'dbus-send --session --type=method_call --dest=com.squigit.app /com/squigit/app com.squigit.app.Capture >/dev/null 2>&1 || busctl --user call com.squigit.app /com/squigit/app com.squigit.app Capture >/dev/null 2>&1 || gdbus call --session --dest com.squigit.app --object-path /com/squigit/app --method com.squigit.app.Capture >/dev/null 2>&1'
```

1.  Go to your system **Settings -\> Keyboard -\> View and Customize Shortcuts**.
2.  Create a new custom shortcut.
3.  Paste the command above into the "Command" field.
4.  Set the binding to `Super+Shift+A` (or whatever you prefer).

Alternatively, you can bind this command using third-party tools like `sxhkd`, `input-remapper`, or `xbindkeys` (though `xbindkeys` is unreliable on Wayland).

</details>

## 2. Quick Start

Once you agree to this guide, the login button will activate. Continue with Google to set up your local profile, and you're ready to go!

- **Capture & Upload:** Use your shortcut, press `Ctrl+V` to paste, or simply drag and drop an image into the app.
- **On-Device OCR:** Squigit extracts text locally on your machine. You can download additional language models in **Settings -> Models**.
- **AI & Reverse Search:** To unlock AI overviews and reverse image search, configure your Bring Your Own Key (BYOK) setup in **Settings -> API Keys**.
- **Make it Yours:** Tailor your AI's responses and behavior in **Settings -> Personalization**.

## 3. Security & Privacy

Because Squigit analyzes your screen, your privacy is our absolute highest priority. We use a zero-trust architecture:

- **Local First:** Your images, chats, and data never leave your device unless you explicitly trigger an AI feature.
- **Encrypted Keys:** Your API keys are hashed and stored locally using AES-256 encryption. We cannot read them, ever.
- **Stateless API Requests:** When you use AI features, requests are sent directly to your provider statelessly. Providers do not get your full context or history.
- **Google OAuth:** We use Google sign-in purely for local profile isolation. We only fetch your account name and avatar to personalize your app experience.
- **⚠️ Lens Feature Warning:** Reverse image search uses ImgBB as a temporary, free image host to process the search. **Do not use the Lens feature for images containing sensitive personal data.**

## 4. Help & Support

Need assistance or want to report a bug? Head over to **Settings -> Help & Support**.

There, you can view your system diagnostics, report bugs, visit our GitHub, or contact us directly. Note: When you contact support, some basic system information (like your OS, Squigit version, and backend engine status) may be sent to help us troubleshoot your issue faster, subject to our **[Privacy Policy](https://github.com/a7mddra/squigit/blob/main/docs/06-policies/SECURITY.md)**.
