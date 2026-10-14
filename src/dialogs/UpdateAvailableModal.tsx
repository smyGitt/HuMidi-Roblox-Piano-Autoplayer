import { openUrl } from "@tauri-apps/plugin-opener";
import { GithubLogoIcon } from "@phosphor-icons/react";
import { MessageModal } from "../components/MessageModal";

export const SOURCEFORGE_URL = "https://sourceforge.net/projects/humidi-roblox-piano-autoplayer/";

const MODAL_WIDTH = 420;
const MODAL_HEIGHT = 224;

interface UpdateAvailableModalProps {
  currentVersion: string;
  latestVersion: string;
  releasesUrl: string;
  onDismiss: () => void;
}

export function UpdateAvailableModal({
  currentVersion,
  latestVersion,
  releasesUrl,
  onDismiss,
}: UpdateAvailableModalProps) {
  return (
    <MessageModal
      title="Update available"
      width={MODAL_WIDTH}
      height={MODAL_HEIGHT}
      onClose={onDismiss}
      body={`A newer version is available (current: ${currentVersion}, latest: ${latestVersion}). Where would you like to get it?`}
      buttons={[
        {
          label: "GitHub",
          variant: "accent",
          leadingIcon: GithubLogoIcon,
          onClick: () => void openUrl(releasesUrl),
        },
        {
          label: "SourceForge",
          onClick: () => void openUrl(SOURCEFORGE_URL),
        },
        {
          label: "Cancel",
          onClick: onDismiss,
        },
      ]}
    />
  );
}
