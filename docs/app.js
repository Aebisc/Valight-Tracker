document.addEventListener("DOMContentLoaded", () => {
  const GITHUB_REPO = "Aebisc/Valight-Tracker";
  const DEFAULT_EXE_URL = `https://github.com/${GITHUB_REPO}/releases/latest/download/VaLight.Tracker_1.0.0_x64-setup.exe`;
  const FALLBACK_VERSION = "v1.0.0";
  const FALLBACK_SIZE = "32 MB";

  const downloadBtns = document.querySelectorAll(".dynamic-download-btn");
  const versionTags = document.querySelectorAll(".dynamic-version-tag");
  const metaSizes = document.querySelectorAll(".dynamic-size-tag");

  function formatBytes(bytes) {
    if (!bytes || bytes === 0) return "32 MB";
    const mb = bytes / (1024 * 1024);
    return `${mb.toFixed(1)} MB`;
  }

  // Fetch the latest release metadata from GitHub
  async function fetchLatestRelease() {
    try {
      const response = await fetch(`https://api.github.com/repos/${GITHUB_REPO}/releases/latest`, {
        headers: { "Accept": "application/vnd.github.v3+json" }
      });

      if (!response.ok) {
        throw new Error(`GitHub API returned ${response.status}`);
      }

      const data = await response.json();
      const version = data.tag_name || FALLBACK_VERSION;
      
      // Find the Windows setup .exe in assets
      let exeAsset = null;
      if (Array.isArray(data.assets)) {
        exeAsset = data.assets.find(asset => 
          asset.name.endsWith(".exe") && !asset.name.endsWith(".sig")
        );
      }

      const downloadUrl = exeAsset ? exeAsset.browser_download_url : DEFAULT_EXE_URL;
      const sizeFormatted = exeAsset ? formatBytes(exeAsset.size) : FALLBACK_SIZE;

      // Update all download buttons and metadata badges
      downloadBtns.forEach(btn => {
        btn.href = downloadUrl;
      });

      versionTags.forEach(el => {
        el.textContent = version;
      });

      metaSizes.forEach(el => {
        el.textContent = sizeFormatted;
      });
    } catch (err) {
      console.warn("Could not fetch release from GitHub API, using fallback defaults:", err);
      // Fallbacks are already set in HTML
    }
  }

  fetchLatestRelease();
});
