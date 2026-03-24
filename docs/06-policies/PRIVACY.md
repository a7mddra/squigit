# Privacy Policy

**Last Updated:** March 24, 2026

Squigit is an open-source, local-first desktop application. We believe your data belongs to you. This Privacy Policy explains how we handle your information, specifically regarding our use of Google APIs and third-party integrations.

## Google API Services User Data Policy

Squigit's use and transfer of information received from Google APIs to any other app will adhere to the [Google API Services User Data Policy](https://developers.google.com/terms/api-services-user-data-policy), including the Limited Use requirements.

### What data we collect and why

When you sign in using Google OAuth, Squigit requests access to your basic profile information (Name, Email, and Avatar).

- **Purpose:** This data is used **exclusively** to create a local profile on your machine to personalize the application interface (e.g., displaying your avatar and name).
- **Data Storage:** This information is stored locally on your device. It is **never** transmitted to, stored on, or processed by any servers controlled by the Squigit developers.

## Local-First & Zero-Trust Architecture

Because Squigit analyzes your screen and text, your privacy is our highest priority:

- **No Telemetry or Backend:** Squigit has no backend database or central server. Your conversations, images, and history never leave your local machine unless you explicitly trigger an AI feature.
- **Encrypted API Keys:** Squigit operates on a Bring Your Own Key (BYOK) model. Your API keys (e.g., Google AI Studio, ImgBB) are hashed and stored locally using AES-256 encryption. We cannot read them, ever.
- **Stateless API Requests:** When you use AI features, requests are sent directly from your machine to your chosen provider (Google or ImgBB). There is no middleman server intercepting or logging your prompts and completions.

## Third-Party Integrations & Google Lens Feature

Squigit includes a reverse image search feature utilizing Google Lens.

- **How it works:** To send a local image to the web-based Google Lens service, Squigit uses the ImgBB API as a temporary image host.
- **Warning:** If you use this specific feature, your selected screenshot will be uploaded to ImgBB and will be accessible via a public URL to process the search. **Do not use the Lens feature for images containing sensitive personal or confidential data.**

## Support and Diagnostics

If you choose to contact our support team or submit a bug report via GitHub, basic system information (such as your OS, Squigit version, and backend engine status) may be included to help us troubleshoot. You have full control over what is sent in these reports.

## Contact

If you have any questions about this Privacy Policy, please open an issue on our [GitHub repository](https://github.com/a7mddra/squigit).
