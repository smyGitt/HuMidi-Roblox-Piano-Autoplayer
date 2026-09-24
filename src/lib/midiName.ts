const MIDI_EXTENSION = /\.midi?$/i;

export function stripMidiExtension(fileName: string): string {
  return fileName.replace(MIDI_EXTENSION, "");
}
