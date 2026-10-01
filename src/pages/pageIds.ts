export const PAGE_IDS = ["playback", "visualizer", "translator", "settings", "debug", "license"] as const;

export type PageId = (typeof PAGE_IDS)[number];
