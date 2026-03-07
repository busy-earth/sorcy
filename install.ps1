$ErrorActionPreference = "Stop"

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "cargo is required to install sorcy. Install Rust from https://rustup.rs and run this script again."
}

$RepoUrl = if ($env:SORCY_REPO_URL) { $env:SORCY_REPO_URL } else { "https://github.com/busy-earth/sorcy" }
$VersionTag = $env:SORCY_VERSION

if ($VersionTag) {
    Write-Host "Installing sorcy from $RepoUrl (tag: $VersionTag)..."
    cargo install --locked --git $RepoUrl --tag $VersionTag --package sorcy
}
else {
    Write-Host "Installing sorcy from $RepoUrl (default branch)..."
    cargo install --locked --git $RepoUrl --package sorcy
}

Write-Host "sorcy installed. Make sure %USERPROFILE%\.cargo\bin is on PATH."
