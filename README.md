# HuMidi

[![Static Badge](https://img.shields.io/badge/SourceForge-HuMidi_v2.0-orange?logo=sourceforge)](https://sourceforge.net/projects/humidi-roblox-piano-autoplayer/)
[![Static Badge](https://img.shields.io/badge/Releases-HuMidi_v2.0-blue?logo=github)](https://github.com/smyGitt/HuMidi-Roblox-Piano-Autoplayer/releases)
![GitHub Repo stars](https://img.shields.io/github/stars/smyGitt/HuMidi-Roblox-Piano-Autoplayer?style=flat)
[![Download HuMidi](https://img.shields.io/sourceforge/dt/humidi-roblox-piano-autoplayer.svg)](https://sourceforge.net/projects/humidi-roblox-piano-autoplayer/files/latest/download)
[![Download HuMidi](https://img.shields.io/sourceforge/dw/humidi-roblox-piano-autoplayer.svg)](https://sourceforge.net/projects/humidi-roblox-piano-autoplayer/files/latest/download)
[![](https://dcbadge.limes.pink/api/server/https://discord.gg/bRaXP9gYZN?style=flat)](https://discord.gg/bRaXP9gYZN)
### You can download the latest version of HuMidi from the Releases or from Sourceforge.
### If you have any issues with HuMidi, please create a new issue here or in the Discord server. Thank you!



## About HuMidi
HuMidi includes various humanization options for a natural, human-like playback. Including automatic pedal timing and 88-key keyboard support. Easy to run via executable or source code, with GUI support included.


<img width="326" height="396" alt="HuMidi v2.0 Expanded view with all Humanization and related options in playback" src="https://github.com/user-attachments/assets/e253ae9e-9280-4cb9-97bb-4ad6ac302237" />
<img width="326" height="396" alt="A minimized, compact view of HuMidi v2.0. Only necessary buttons are shown." src="https://github.com/user-attachments/assets/11065766-a107-461a-a46f-84f8119098cf" />
<img width="180" height="326" alt="HuMidi v2.0 with a new Light mode theme, showing a settings page with option to change color theme." src="https://github.com/user-attachments/assets/344cf20a-ffe2-482c-bc71-53c4c378a5ad" />

## Note on ML Components
HuMidi now uses a trained model from [PedalAI](https://github.com/smyGitt/PedalAI) for sustain pedal generation. For information on training data sources, licensing, and related legal considerations, refer to [PedalAI's README](https://github.com/smyGitt/PedalAI#copyright--intended-use).

# How do you run it?
### If you downloaded the file from the releases:
  just run the `.exe` or `.dmg`! 

### Or, if you don't trust me...
You can build the installer yourself with Tauri. Install what is listed under Dependencies first, then open a command prompt in the project folder and type this.

    npm ci
    npm run tauri build

you will find the installer in `src-tauri/target/release/bundle`.

### ...if you choose to do neither above:
You need to run this in a command prompt. After you navigate to the project folder, type this.

    npm ci
    npm run tauri dev

Remember that it accepts `.mid` files only. it works best with piano-only `.mid`, but I've seen it work with mixed instruments.

# Dependencies
If you chose to not trust me, no judging, and did not want to use the file in the releases, you need to install a few things first. You need Node.js and Rust. Everything else depends on your system and is listed in the [Tauri prerequisites](https://tauri.app/start/prerequisites/).

On Debian or Ubuntu, this installs the system libraries.

    sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxkbcommon-dev patchelf
