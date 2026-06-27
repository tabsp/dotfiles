#!/usr/bin/env sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_dir=$(CDPATH= cd -- "$script_dir/.." && pwd)

public_dir=${PUBLIC_DIR:-"$repo_dir/public"}
base_url=${DOTFILES_SITE_URL:-"https://dotfiles.tabsp.com"}
release_base_url=${DOTMAN_RELEASE_BASE_URL:-"https://github.com/tabsp/dotfiles/releases/latest/download"}

cd "$repo_dir"

package_version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
git_sha=$(git rev-parse --short HEAD 2>/dev/null || printf 'unknown')
bundle_version=$(git describe --tags --always --dirty 2>/dev/null || printf '%s' "$git_sha")
generated_at=$(date -u '+%Y-%m-%dT%H:%M:%SZ')

rm -rf "$public_dir"
mkdir -p "$public_dir/bundle"
mkdir -p "$public_dir/assets/screenshots"

cp "$repo_dir/scripts/install.sh" "$public_dir/install.sh"
chmod 755 "$public_dir/install.sh"
cp "$repo_dir/assets/screenshots/terminal-preview.png" "$public_dir/assets/screenshots/terminal-preview.png"

git archive --format=tar HEAD -- \
  dotman.yaml \
  dotman.bootstrap.yaml \
  config \
  bin \
  packages \
  README.md \
  README.zh-CN.md \
  docs/new-machine.md |
  gzip -n >"$public_dir/bundle/latest.tar.gz"

if command -v sha256sum >/dev/null 2>&1; then
  bundle_sha256=$(sha256sum "$public_dir/bundle/latest.tar.gz" | awk '{print $1}')
else
  bundle_sha256=$(shasum -a 256 "$public_dir/bundle/latest.tar.gz" | awk '{print $1}')
fi

cat >"$public_dir/manifest.json" <<EOF
{
  "schema": 1,
  "generated_at": "$generated_at",
  "bundle_url": "$base_url/bundle/latest.tar.gz",
  "bundle_sha256": "$bundle_sha256",
  "dotman_version": "$package_version",
  "dotman_release_base_url": "$release_base_url",
  "dotman_asset_template": "dotman-{target}.tar.gz",
  "bundle": {
    "version": "$bundle_version",
    "url": "$base_url/bundle/latest.tar.gz",
    "sha256": "$bundle_sha256"
  },
  "dotman": {
    "version": "$package_version",
    "release_base_url": "$release_base_url",
    "asset_template": "dotman-{target}.tar.gz",
    "targets_note": "Keep this list in sync with .github/workflows/release-dotman.yml.",
    "targets": [
      "aarch64-apple-darwin",
      "x86_64-apple-darwin",
      "x86_64-unknown-linux-gnu",
      "aarch64-unknown-linux-gnu"
    ]
  }
}
EOF

cat >"$public_dir/index.html" <<EOF
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>tabsp dotfiles</title>
    <style>
      :root {
        color-scheme: dark;
        --ink: #f7f3ea;
        --muted: rgba(247, 243, 234, 0.68);
        --line: rgba(247, 243, 234, 0.20);
        --code: rgba(11, 12, 11, 0.84);
      }

      * {
        box-sizing: border-box;
      }

      body {
        margin: 0;
        min-height: 100vh;
        color: var(--ink);
        font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
        background:
          linear-gradient(rgba(0, 0, 0, 0.30), rgba(0, 0, 0, 0.78)),
          url("/assets/screenshots/terminal-preview.png") center / cover fixed;
        display: grid;
        align-items: end;
        padding: 28px;
        line-height: 1.5;
      }

      main {
        width: min(1040px, 100%);
        margin: 0 auto;
        padding: 0 0 46px;
      }

      .eyebrow {
        margin: 0 0 12px;
        color: var(--muted);
        font-size: 0.82rem;
        font-weight: 700;
        text-transform: uppercase;
      }

      h1 {
        margin: 0;
        font-size: clamp(3rem, 8vw, 6.75rem);
        line-height: 0.92;
        letter-spacing: 0;
      }

      p {
        max-width: 620px;
        margin: 18px 0 0;
        color: var(--muted);
        font-size: clamp(1rem, 2vw, 1.18rem);
      }

      .command {
        width: min(720px, 100%);
        margin: 28px 0 0;
        overflow-x: auto;
        border: 1px solid var(--line);
        border-radius: 8px;
        background: var(--code);
        color: var(--ink);
        position: relative;
      }

      pre {
        margin: 0;
        padding: 18px 112px 18px 20px;
        font: 0.98rem / 1.6 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
        white-space: pre;
      }

      button {
        position: absolute;
        top: 10px;
        right: 10px;
        height: 34px;
        padding: 0 14px;
        border: 1px solid var(--line);
        border-radius: 6px;
        background: rgba(255, 255, 255, 0.08);
        color: var(--ink);
        cursor: pointer;
        font: 700 0.82rem ui-sans-serif, system-ui, sans-serif;
      }

      button:hover {
        background: rgba(255, 255, 255, 0.14);
      }

      nav {
        display: flex;
        flex-wrap: wrap;
        gap: 18px;
        margin-top: 22px;
      }

      a {
        color: var(--ink);
        font-weight: 700;
        text-underline-offset: 4px;
      }

      @media (max-width: 760px) {
        body {
          background-attachment: scroll;
          padding: 22px;
        }

        main {
          padding-bottom: 28px;
        }

        pre {
          padding-right: 20px;
          padding-bottom: 58px;
        }

        button {
          top: auto;
          right: 10px;
          bottom: 10px;
        }
      }
    </style>
  </head>
  <body>
    <main>
      <p class="eyebrow">dotman powered</p>
      <h1>tabsp dotfiles</h1>
      <p>Install dotman and the latest dotfiles bundle without cloning or building on the target machine.</p>
      <div class="command">
        <pre><code id="install-command">curl -fsSL $base_url/install.sh | sh</code></pre>
        <button type="button" id="copy-command">Copy</button>
      </div>
      <nav aria-label="Published files">
        <a href="/install.sh">install.sh</a>
        <a href="/manifest.json">manifest.json</a>
        <a href="/bundle/latest.tar.gz">bundle</a>
      </nav>
    </main>
    <script>
      const button = document.getElementById("copy-command");
      const command = document.getElementById("install-command").textContent;

      button.addEventListener("click", async () => {
        try {
          await navigator.clipboard.writeText(command);
          button.textContent = "Copied";
        } catch {
          button.textContent = "Select";
        }

        window.setTimeout(() => {
          button.textContent = "Copy";
        }, 1600);
      });
    </script>
  </body>
</html>
EOF

printf 'built site assets in %s\n' "$public_dir"
