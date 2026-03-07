# Bring Your Own Key (BYOK)

This project operates on a **Bring Your Own Key (BYOK)** model. This implies that we do not resell API access or add markups to model usage. Instead, you plug in your own API keys from **Google AI Studio** and **ImgBB**, and the application acts as a direct client.

This approach is built on five core pillars: **Security, Cost-Efficiency, Performance, Flexibility, and Transparency.**

## 1. 🛡️ Security & Privacy

Security is the primary concern when handling API keys. Our architecture ensures your credentials remain under your control.

- **Client-Side Storage:** Your API keys are stored **locally on your device** (using `localStorage` or encrypted browser stores). They are **never** sent to our servers or a third-party backend.

- **Direct Communication:** Requests go directly from your browser/client to the API Provider (Google or ImgBB). There is no "middleman" server intercepting or logging your prompts and completions.

- **Encryption:** Keys are encrypted at rest within the local environment.

- **Revocability:** Since you own the key, you can revoke access instantly via your provider's dashboard if you suspect any issues.

## 2. 💸 Cost-Effective (True "Free" Tier)

By cutting out the middleman, you pay the wholesale price (or often nothing at all).

- **Leverage Free Tiers:** Google AI Studio offers a generous free tier for Gemini models, and ImgBB provides free API access for image hosting. You can often use this app completely free of charge.

- **No Markups:** If you exceed free limits, you pay the direct provider rates without any SaaS subscription markup.

- **Direct Billing:** You manage your usage limits directly with Google and ImgBB.

## 3. ⚡ Speed & Latency

The BYOK architecture is designed for minimal latency.

- **Zero-Hop Routing:** Traditional apps route your request: `You -> App Server -> AI Provider -> App Server -> You`.

- **Direct Connection:** We route: `You -> Google/ImgBB -> You`.

- **Streaming First:** Text responses from Gemini are streamed directly to your interface in real-time.

## 4. 🔗 Ecosystem Flexibility

We support the full range of modern Gemini models and essential media tools.

- **Model Switching:** Instantly hot-swap between **Gemini 3.1 Flash** (for efficiency) and **Gemini 3.1 Pro** (for reasoning-heavy tasks).

- **Image Injection:** We integrate **ImgBB** to handle image uploads. This allows you to upload images via the API, get a direct URL, and inject them into the **Google Lens official service** to get you ready results in your default browser with a single button press.

## 5. 🔓 Open Source Transparency

Trust is earned through verification.

- **Audit the Code:** You don't have to take our word for it. You can inspect the network requests in your browser's "Network" tab to confirm keys are only being sent to `generativelanguage.googleapis.com` (Google) and `api.imgbb.com` (ImgBB).

## Supported Providers & Setup

To get started, generate keys from the supported providers below:

| **Provider** | **Service / Models**                              | **Get Key**                                                |
| ------------ | ------------------------------------------------- | ---------------------------------------------------------- |
| **Google**   | **LLM:** Gemini 3.1 Flash, 3.1 Pro                | [Google AI Studio](https://aistudio.google.com/app/apikey) |
| **ImgBB**    | **Image Hosting:** For uploading/analyzing images | [ImgBB API](https://api.imgbb.com/)                        |

### How to add your keys (Automated Flow)

1. Navigate to **Settings** > **Reset API Keys** within the app.

2. Click the setup button for your desired provider (Google or ImgBB). This will launch the official provider website in your browser and start the **Secure Clipboard Watcher**.

3. Generate your API key on the provider's site and simply **copy it to your clipboard**.

4. The application will automatically detect the key pattern (e.g., `AIzaS...` for Google), stop the watcher, and securely encrypt/save the key. You do not need to paste it manually.
