import { useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ArrowSquareOutIcon, GithubLogoIcon } from "@phosphor-icons/react";
import { MessageModal } from "../components/MessageModal";
import { downloadAndInstallUpdate } from "../lib/tauri";

const SOURCEFORGE_URL = "https://sourceforge.net/projects/humidi-roblox-piano-autoplayer/";

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
  const [updating, setUpdating] = useState(false);
  const [error, setError] = useState(false);

  async function handleUpdate() {
    setError(false);
    setUpdating(true);
    try {
      await downloadAndInstallUpdate();
    } catch {
      setError(true);
      setUpdating(false);
    }
  }

  return (
    <MessageModal
      title="Update available"
      width={460}
      height={260}
      body={
        <>
          Update available from current version ({currentVersion}) to the latest version ({latestVersion}). Would
          you like to download it now, manually download (clicking the button will take you to the latest release
          page) or cancel the update?
          {error && (
            <>
              <br />
              Couldn't download the update. Try again or download it manually.
            </>
          )}
        </>
      }
      buttons={[
        {
          label: updating ? "Updating..." : "Update",
          variant: "accent",
          disabled: updating,
          onClick: () => void handleUpdate(),
        },
        {
          label: "To GitHub Releases",
          leadingIcon: GithubLogoIcon,
          trailingIcon: ArrowSquareOutIcon,
          iconSize: 32,
          disabled: updating,
          onClick: () => void openUrl(releasesUrl),
        },
        {
          label: "To SourceForge",
          trailingIcon: ArrowSquareOutIcon,
          iconSize: 32,
          disabled: updating,
          onClick: () => void openUrl(SOURCEFORGE_URL),
        },
        {
          label: "Cancel",
          disabled: updating,
          onClick: onDismiss,
        },
      ]}
    />
  );
}
