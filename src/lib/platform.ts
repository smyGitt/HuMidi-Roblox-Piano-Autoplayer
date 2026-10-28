const LINUX_MARKER = "Linux";

export function isLinux(): boolean {
  return typeof navigator !== "undefined" && navigator.userAgent.includes(LINUX_MARKER);
}
