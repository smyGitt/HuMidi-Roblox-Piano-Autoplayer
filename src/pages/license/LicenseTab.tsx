import { useState } from "react";
import { TabPage } from "../../components/TabPage";

const MIT_LICENSE = (year: string, holder: string) => `MIT License

Copyright (c) ${year} ${holder}

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
`;

const LICENSE_TEXTS: Record<string, string> = {
  HuMidi: MIT_LICENSE("2026", "smyGitt"),

  "PedalAI Dataset": `The following datasets were used to train the BiLSTM AI pedal timing
model bundled with HuMidi.

----------------
POP909
----------------
A piano MIDI dataset of 909 popular songs with performance annotations.

Citation:
  Wang, Z., Chen, K., Jiang, J., Zhang, Y., Xu, M., Dai, S., Xia, G.,
  & Fazekas, G. (2020). POP909: A Pop-song Dataset for Music Arrangement
  Generation. Proceedings of ISMIR 2020.

License : MIT
URL     : https://github.com/music-x-lab/POP909-Dataset

----------------
GiantMIDI-Piano
----------------
A large-scale MIDI dataset of classical piano music transcribed from
audio recordings.

Citation:
  Kong, Q., Li, B., Chen, J., & Wang, Y. (2020). GiantMIDI-Piano: A
  large-scale MIDI dataset for classical piano music. arXiv:2010.07061.

License : Creative Commons Attribution 4.0 International (CC BY 4.0)

  You are free to share and adapt the material for any purpose, provided
  appropriate credit is given.

URL     : https://github.com/bytedance/GiantMIDI-Piano
`,

  "Third-Party Libraries": `tauri
  License : MIT OR Apache-2.0
  URL     : https://github.com/tauri-apps/tauri

tauri-plugin-dialog / tauri-plugin-opener / tauri-plugin-updater / tauri-plugin-global-shortcut
  License : MIT OR Apache-2.0
  URL     : https://github.com/tauri-apps/plugins-workspace

react / react-dom
  License : MIT
  URL     : https://react.dev

@tauri-apps/api
  License : MIT OR Apache-2.0
  URL     : https://github.com/tauri-apps/tauri

serde / serde_json
  License : MIT OR Apache-2.0
  URL     : https://github.com/serde-rs

candle-core
  License : MIT OR Apache-2.0
  URL     : https://github.com/huggingface/candle

enigo
  License : MIT
  URL     : https://github.com/enigo-rs/enigo

rdev
  License : MIT
  URL     : https://github.com/Narsil/rdev

midly
  License : Unlicense
  URL     : https://github.com/negamartin/midly

dirs / rand / chrono
  License : MIT OR Apache-2.0
  URL     : https://github.com/soc/dirs-rs, https://github.com/rust-random/rand, https://github.com/chronotope/chrono
`,

  "Phosphor Icons": MIT_LICENSE("2020", "Phosphor Icons"),
};

const NAV_ITEMS = Object.keys(LICENSE_TEXTS);

export function LicenseTab() {
  const [active, setActive] = useState(0);

  return (
    <div className="license-tab">
      <TabPage tabs={NAV_ITEMS} active={active} onChange={setActive}>
        <div className="license-tab__text">{LICENSE_TEXTS[NAV_ITEMS[active]]}</div>
      </TabPage>
    </div>
  );
}
