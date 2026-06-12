# gitid installer for Windows (PowerShell 5+).
#
#   irm https://raw.githubusercontent.com/dgreco-at-speer/gitid/main/scripts/install.ps1 | iex
#
# Environment overrides:
#   GITID_REPO         owner/repo to install from (default below)
#   GITID_VERSION      release tag to install (default: latest release)
#   GITID_INSTALL_DIR  where to put gitid.exe (default: %LOCALAPPDATA%\gitid\bin)
#   GH_TOKEN / GITHUB_TOKEN   token for private-repo downloads (when gh is absent)
#
# Private repos: install GitHub CLI and run `gh auth login` first.
#Requires -Version 5
$ErrorActionPreference = 'Stop'

$Repo = if ($env:GITID_REPO) { $env:GITID_REPO } else { 'dgreco-at-speer/gitid' }
$Version = $env:GITID_VERSION
$InstallDir = if ($env:GITID_INSTALL_DIR) { $env:GITID_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'gitid\bin' }
$Target = 'x86_64-pc-windows-gnu'
$Token = if ($env:GH_TOKEN) { $env:GH_TOKEN } elseif ($env:GITHUB_TOKEN) { $env:GITHUB_TOKEN } else { $null }

$tmp = New-Item -ItemType Directory -Path (Join-Path $env:TEMP ("gitid-" + [guid]::NewGuid()))
try {
  if (Get-Command gh -ErrorAction SilentlyContinue) {
    Write-Host "Downloading gitid ($(if ($Version) { $Version } else { 'latest' })) via gh..."
    $ghArgs = @('release', 'download')
    if ($Version) { $ghArgs += $Version }
    $ghArgs += @('--repo', $Repo, '--pattern', "gitid-*-$Target.zip", '--dir', $tmp.FullName)
    & gh @ghArgs
  }
  else {
    $headers = @{}
    if ($Token) { $headers['Authorization'] = "Bearer $Token" }
    if (-not $Version) {
      Write-Host "Resolving latest release..."
      $rel = Invoke-RestMethod -Headers $headers "https://api.github.com/repos/$Repo/releases/latest"
      $Version = $rel.tag_name
    }
    $asset = "gitid-$Version-$Target.zip"
    $url = "https://github.com/$Repo/releases/download/$Version/$asset"
    Write-Host "Downloading $asset..."
    Invoke-WebRequest -Headers $headers -Uri $url -OutFile (Join-Path $tmp.FullName $asset)
  }

  $zip = Get-ChildItem -Path $tmp.FullName -Filter *.zip | Select-Object -First 1
  if (-not $zip) { throw "no archive downloaded" }
  Expand-Archive -Path $zip.FullName -DestinationPath $tmp.FullName -Force
  $bin = Get-ChildItem -Path $tmp.FullName -Recurse -Filter gitid.exe | Select-Object -First 1
  if (-not $bin) { throw "gitid.exe not found in archive" }

  New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
  Copy-Item $bin.FullName (Join-Path $InstallDir 'gitid.exe') -Force
  Write-Host "Installed gitid to $InstallDir\gitid.exe"

  $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
  if ($userPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable('Path', "$userPath;$InstallDir", 'User')
    Write-Host "Added $InstallDir to your user PATH (restart your shell to pick it up)."
  }
  Write-Host "Next: run 'gitid setup' to install the shell hook."
}
finally {
  Remove-Item -Recurse -Force $tmp.FullName
}
