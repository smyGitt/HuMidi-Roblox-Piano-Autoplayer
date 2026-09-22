import { openUrl } from "@tauri-apps/plugin-opener";

interface UpdateAvailableToastProps {
  tag: string;
  url: string;
  onDismiss: () => void;
}

export function UpdateAvailableToast({ tag, url, onDismiss }: UpdateAvailableToastProps) {
  return (
    <div className="update-toast">
      <span className="update-toast__msg">Update available: {tag}</span>
      <button className="update-toast__btn update-toast__btn--accent" onClick={() => void openUrl(url)}>
        Open
      </button>
      <button className="update-toast__btn" onClick={onDismiss}>
        Dismiss
      </button>
    </div>
  );
}
