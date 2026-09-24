const MIDI_ACCEPT = ".mid,.midi";

export function pickMidiFile(onPicked: (name: string) => void) {
  const input = document.createElement("input");
  input.type = "file";
  input.accept = MIDI_ACCEPT;
  input.onchange = () => {
    const file = input.files?.[0];
    if (file) onPicked(file.name);
  };
  input.click();
}
