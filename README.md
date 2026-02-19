# JDA - Jyotismoy's Download Accelerator

<p align="center">
  <img src="src-tauri/icons/128x128.png" width="128" height="128" alt="JDA Logo">
  <br>
  <b>High-performance, multi-threaded download management built with Rust.</b>
</p>

---

##  Overview

**JDA** (Jyotismoy's Download Accelerator) is a modern download engine designed to maximize network throughput. Unlike standard browser download managers, JDA utilizes a **"Stubborn Worker" architecture**—insisting on multi-threaded parallel chunking even when server headers are ambiguous, ensuring you get the most out of your bandwidth.

Built using **Rust** for the core engine and **Tauri** for a lightweight, native GUI experience, JDA is fast, memory-efficient, and privacy-focused.

## Key Features

* **Multi-Threaded Engine:** Splits files into dynamic, parallel chunks to saturate your connection.
* **"Stubborn Worker" Logic:** Advanced range-request detection that attempts parallel downloads even when servers don't explicitly broadcast support.
* **Deep Link Integration:** Seamlessly intercepts downloads from Chrome/Edge via the custom `jda://` protocol.
* **Smart Interception Toggle:** Control exactly when the accelerator is active via the companion browser extension.
* **Pause & Resume:** Robust state management for interrupted or long-running downloads.
* **Native Performance:** Zero-cost abstractions provided by Rust with a tiny resource footprint.

## Tech Stack

* **Backend:** [Rust](https://www.rust-lang.org/) (Core Logic & Tauri)
* **Frontend:** [React](https://reactjs.org/)
* **Desktop Framework:** [Tauri](https://tauri.app/) (v2.0)
* **Communication:** Custom Protocol Handler (`jda://`)
* **Storage:** `chrome.storage` (Extension side) & Local JSON (App side)

## Getting Started

### Prerequisites

* **Rust:** [Install Rust](https://www.rust-lang.org/tools/install)
* **Node.js:** [Install Node.js](https://nodejs.org/) (v18 or later)
* **WebView2:** (usually pre-installed)

### Installation & Development

1. **Clone the Repo:**

    ```bash
    git clone [https://github.com/JyotismoyK/jda.git](https://github.com/JyotismoyK/jda.git)
    cd jda
    ```

2. **Install Frontend Dependencies:**

    ```bash
    npm install
    ```

3. **Run in Development Mode:**

    ```bash
    cargo-tauri dev
    ```

4. **Build Production Installer:**

    ```bash
    cargo-tauri build
    ```

## Browser Integration

To unlock the full potential of JDA, use the [JDA Browser Extension](https://github.com/JyotismoyKalita/JDA-Extension):

1. Load the extension from the `src` directory (unpacked mode).
2. Toggle the accelerator to **Active**.
3. Clicking any download link in your browser will now automatically trigger JDA via the deep-link protocol.

## Preview

![Preview](/media/preview.gif)

## Roadmap

* **Batch Link Grabbing:** Extract and queue all links from a webpage.
* **Speed Limiter:** Manually cap bandwidth for background tasks.
* **Queue Scheduling:** Set specific times for heavy downloads to start.

## Contributing

As a project in active development, contributions are highly encouraged!

1. Fork the project.
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`).
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`).
4. Push to the branch (`git push origin feature/AmazingFeature`).
5. Open a Pull Request.

## License

Distributed under the MIT License. See `LICENSE` for more information.

---

## Author

[Jyotismoy Kalita](https://github.com/JyotismoyK)