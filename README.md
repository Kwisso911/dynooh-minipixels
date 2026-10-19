# Dynooh MiniPixels

**Author and maintainer: Christian Tanner** ([Kwisso911](https://github.com/Kwisso911))

A small, fast image and media viewer for Windows. Open images, audio and video in tabs, tiles, a list or separate windows – with rulers, guides and a pixel grid for precise checks.

**Download:** [apps.dynooh.com](https://apps.dynooh.com/) (portable `.exe`, no installation)

## Features

- **Images** – PNG, JPG, BMP, GIF and TIF with zoom, fit and pixel-exact view
- **Audio and video** – MP3, MP4 and MOV, codec shown per file; unsupported video codecs can be converted to H.264 with FFmpeg (downloaded on request)
- **Playback bar** – one player for all views: playback keeps running when you switch views, plays straight from tiles and list, optionally moves on to the next file
- **Video like images** – zoom, rulers, guides and pixel grid also for a playing video
- **Slideshow** – F5 starts and stops it, seconds per image in the settings
- **Open your way** – new window, tabs, tiles or list, browse a whole folder
- **Folder browsing** – ← / → load the previous or next file of the shown file's folder in place, Ctrl+Tab switches between the open files
- **Sorting** – by name or age, or your own order by dragging tabs, tiles and rows
- **Rulers and guides** – Ctrl+R shows rulers, drag guides out of them, Ctrl+H hides them, Del removes the selected guide
- **Pixel grid** – fine grid around every pixel above 400 % zoom
- **Clipboard** – paste images or copied files with Ctrl+V
- **Your layout** – light or dark design, English or German interface

## Build from source

Requirements: Windows 10/11, [Node.js](https://nodejs.org/) 20+, [Rust](https://rustup.rs/) (stable), and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

```sh
npm ci
npx tauri dev                 # development
npx tauri build --no-bundle   # portable exe in src-tauri/target/release/
```

## Code signing policy

Code signing certificate: [Certum Open Source Code Signing](https://www.certum.eu/) (Asseco Data Systems), issued to Christian Tanner. All releases are signed.

- Committers and reviewers: Christian Tanner ([Kwisso911](https://github.com/Kwisso911))
- Approvers: Christian Tanner ([Kwisso911](https://github.com/Kwisso911))

Release binaries are built by the maintainer from tagged source code in this repository (`npm run release`) and signed with the Certum certificate including a timestamp. Only the maintainer can sign; the private key is held in the Certum SimplySign cloud.

## Privacy

This program does not collect or transmit personal data. It only goes online when you ask it to download FFmpeg for converting a video: it then queries the GitHub API for the latest [BtbN FFmpeg build](https://github.com/BtbN/FFmpeg-Builds) (LGPL) and downloads it from GitHub. Files are never uploaded. Full privacy policy: [apps.dynooh.com/privacy.html](https://apps.dynooh.com/privacy.html)

## License

[MIT](LICENSE)
