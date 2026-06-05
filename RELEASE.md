# Release Guide

Step-by-step commands to cut a new GitSearcher release.

Replace `X.Y.Z` with the new version (e.g. `0.1.1`) everywhere below.

> Prerequisites: .NET SDK, Rust toolchain (`cargo`), Inno Setup 6 (`ISCC.exe`),
> and (for publishing) the GitHub CLI (`gh`).

---

## 1. Bump the version

Update the version string in all three places:

- `GitSearcher.csproj` → `<Version>X.Y.Z</Version>`
- `installer/GitSearcher.iss` → `#define MyAppVersion "X.Y.Z"`
- `ui/Cargo.toml` → `version = "X.Y.Z"`

Then refresh the Rust lockfile:

```powershell
cargo update -p gitsearcher-ui --manifest-path ui/Cargo.toml
```

---

## 2. Build the binaries

C# CLI (self-contained, single file — no .NET required on the target machine):

```powershell
dotnet publish -c Release -r win-x64 --self-contained true -p:PublishSingleFile=true -o publish/win-x64
```

Rust GUI (optimized release):

```powershell
cargo build --release --manifest-path ui/Cargo.toml
```

Optional — Linux CLI build (for attaching to the GitHub Release):

```powershell
dotnet publish -c Release -r linux-x64 --self-contained true -p:PublishSingleFile=true -o publish/linux-x64
```

---

## 3. Build the installer

The Inno Setup script bundles the CLI, the GUI and the default config, and
writes `installer-output/GitSearcher-Setup-X.Y.Z.exe`.

```powershell
& "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe" installer\GitSearcher.iss
```

Compute the SHA256 (needed for the winget manifest):

```powershell
(Get-FileHash .\installer-output\GitSearcher-Setup-X.Y.Z.exe -Algorithm SHA256).Hash
```

---

## 4. Commit and tag

```powershell
git add GitSearcher.csproj installer/GitSearcher.iss ui/Cargo.toml ui/Cargo.lock
git commit -m "Bump version to X.Y.Z"

git tag -a vX.Y.Z -m "GitSearcher vX.Y.Z"

# Push the commit and the tag (ask before pushing if working with others)
git push origin master
git push origin vX.Y.Z
```

To delete / move a tag if you made a mistake:

```powershell
git tag -d vX.Y.Z
git push origin :refs/tags/vX.Y.Z
```

---

## 5. Create the GitHub Release

Attach the installer (and optionally the raw binaries). The release tag must
match the git tag from step 4.

```powershell
gh release create vX.Y.Z `
  --title "GitSearcher vX.Y.Z" `
  --notes "Release notes here." `
  installer-output\GitSearcher-Setup-X.Y.Z.exe `
  publish\win-x64\GitSearcher.exe
```

To attach extra assets to an existing release:

```powershell
gh release upload vX.Y.Z publish\linux-x64\GitSearcher
```

---

## 6. Update the winget manifest (optional)

Manifests live under `manifests/manifests/p/Parresia/GitSearcher/<version>/`.
Easiest path is to let `wingetcreate` update URL + SHA256 from the release:

```powershell
winget install Microsoft.WingetCreate   # first time only

wingetcreate update Parresia.GitSearcher `
  --version X.Y.Z `
  --urls https://github.com/elguala9/GitSearcher/releases/download/vX.Y.Z/GitSearcher-Setup-X.Y.Z.exe `
  --submit
```

`--submit` opens the PR against `microsoft/winget-pkgs` automatically. Drop it
to only generate the manifests locally.

See `todo.md` for the full first-time winget onboarding checklist.
