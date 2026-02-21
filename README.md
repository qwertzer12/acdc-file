# acdc

`acdc` is a Rust CLI/TUI helper for discovering Docker image tags (Docker Hub) and generating container-friendly workflows.

## Install

### From source

```bash
cargo install --path .
```

### Homebrew (after releases are published)

```bash
brew tap qwertzer12/tap
brew install acdc
```

## Usage

Run interactive mode (TUI):

```bash
acdc
```

Run without TUI:

```bash
acdc --console
```

Search tags directly:

```bash
acdc search-tags library nginx "alpine" --limit 15
```

Auto-resolve image and search tags:

```bash
acdc auto-tags nginx "alpine" --limit 15
```

Generate shell completions:

```bash
acdc completions bash > /etc/bash_completion.d/acdc
```

## Release with cargo-dist

This project uses `cargo-dist` with GitHub Releases and Homebrew publishing.

### Why CI failed with “release.yml has out of date contents”

You enabled Homebrew publishing in `dist-workspace.toml`:

- `installers = ["shell", "powershell", "homebrew"]`
- `publish-jobs = ["homebrew"]`
- `tap = "qwertzer12/homebrew-tap"`

When those settings change, `cargo-dist` expects `.github/workflows/release.yml` to be regenerated. CI failed because the checked-in workflow does not yet include the new Homebrew publish job.

### Fix

Run this locally and commit the generated workflow changes:

```bash
dist init
git add .github/workflows/release.yml dist-workspace.toml Cargo.toml
git commit -m "chore(dist): regenerate release workflow for homebrew publishing"
git push
```

After that, rerun the release workflow.

### Required GitHub secret

For pushing formula updates to your tap repo, set:

- `HOMEBREW_TAP_TOKEN` (PAT with write access to `qwertzer12/homebrew-tap`)

`GITHUB_TOKEN` is already provided by GitHub Actions for release creation.

### Optional (not recommended)

You can bypass the out-of-date check with `allow-dirty` in `Cargo.toml`, but this may hide real CI drift. Regenerating with `dist init` is the correct fix.
