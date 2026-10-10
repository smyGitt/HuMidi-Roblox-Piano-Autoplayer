import { openUrl } from "@tauri-apps/plugin-opener";
import { ArrowSquareOutIcon, GithubLogoIcon } from "@phosphor-icons/react";
import { MessageModal } from "../components/MessageModal";

export const SOURCEFORGE_URL = "https://sourceforge.net/projects/humidi-roblox-piano-autoplayer/";

const MODAL_WIDTH = 460;
const MODAL_HEIGHT = 240;

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
          label: "To GitHub Releases",
          variant: "accent",
          leadingIcon: GithubLogoIcon,
          trailingIcon: ArrowSquareOutIcon,
          size: "lg",
          onClick: () => void openUrl(releasesUrl),
        },
        {
          label: "To SourceForge",
          trailingIcon: ArrowSquareOutIcon,
          size: "lg",
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
