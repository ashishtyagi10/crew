<#
.SYNOPSIS
    Crew installer for Windows — no administrator rights required.

.DESCRIPTION
    Installs the prebuilt crew.exe from the latest GitHub release into a
    per-user location and puts it on the user PATH. Nothing is written outside
    the current user's profile: no Program Files, no HKLM, no MSI, no UAC
    prompt. `crew /update` later replaces the same file in place, so upgrading
    never needs an administrator either.

.PARAMETER Version
    Release tag to install (e.g. v0.17.8). Defaults to the latest release.

.PARAMETER InstallDir
    Where crew.exe goes. Defaults to $env:CREW_INSTALL_DIR, else
    %LOCALAPPDATA%\Programs\crew.

.PARAMETER NoPath
    Skip adding the install directory to the user PATH.

.EXAMPLE
    irm https://raw.githubusercontent.com/ashishtyagi10/crew/main/install.ps1 | iex

.EXAMPLE
    # Pin a version, or install somewhere else:
    .\install.ps1 -Version v0.17.8 -InstallDir D:\tools\crew
#>
[CmdletBinding()]
param(
    [string]$Version,
    [string]$InstallDir,
    [switch]$NoPath
)

$ErrorActionPreference = 'Stop'
# Windows PowerShell 5.1 still defaults to TLS 1.0, which github.com refuses.
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$Repo = 'ashishtyagi10/crew'
$BinName = 'crew.exe'

function Write-Step($msg) { Write-Host $msg -ForegroundColor Cyan }
function Write-Note($msg) { Write-Host $msg -ForegroundColor Yellow }

# --- Platform ---------------------------------------------------------------

# PROCESSOR_ARCHITECTURE reports the *process* architecture, so an x86 shell on
# an ARM64 machine would lie; PROCESSOR_ARCHITEW6432 is set in exactly that case.
$archRaw = $env:PROCESSOR_ARCHITEW6432
if (-not $archRaw) { $archRaw = $env:PROCESSOR_ARCHITECTURE }
$arch = switch ($archRaw) {
    'AMD64' { 'x86_64' }
    'ARM64' { 'aarch64' }
    default { throw "Unsupported architecture: $archRaw (crew ships x64 and arm64)" }
}
$target = "$arch-pc-windows-msvc"
Write-Step "Detected platform: $target"

# --- Release ----------------------------------------------------------------

if (-not $Version) {
    $api = "https://api.github.com/repos/$Repo/releases/latest"
    try {
        $Version = (Invoke-RestMethod -Uri $api -Headers @{ 'User-Agent' = 'crew-installer' }).tag_name
    } catch {
        throw "Could not reach the GitHub release API ($api): $($_.Exception.Message)"
    }
}
if (-not $Version) { throw 'Could not determine the latest release.' }
Write-Step "Release: $Version"

$asset = "crew-$Version-$target.zip"
$url = "https://github.com/$Repo/releases/download/$Version/$asset"

# --- Destination ------------------------------------------------------------

if (-not $InstallDir) { $InstallDir = $env:CREW_INSTALL_DIR }
if (-not $InstallDir) { $InstallDir = Join-Path $env:LOCALAPPDATA 'Programs\crew' }
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

# Prove it is writable before downloading, so a bad -InstallDir fails fast.
$probe = Join-Path $InstallDir '.crew-write-test'
try {
    [IO.File]::WriteAllText($probe, '')
    Remove-Item $probe -Force
} catch {
    throw "$InstallDir is not writable. Pass -InstallDir with a directory you own."
}

$tmp = Join-Path ([IO.Path]::GetTempPath()) ("crew-install-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $tmp | Out-Null

try {
    $zip = Join-Path $tmp $asset
    Write-Step "Downloading $url ..."
    Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing

    # --- Verify -------------------------------------------------------------
    # The release publishes one checksums file for every asset. A missing file
    # is not fatal (older releases predate it); a MISMATCH always is.
    try {
        $sumsUrl = "https://github.com/$Repo/releases/download/$Version/checksums-sha256.txt"
        $sums = (Invoke-WebRequest -Uri $sumsUrl -UseBasicParsing).Content
    } catch {
        $sums = $null
    }
    if ($sums) {
        $line = $sums -split "`n" | Where-Object { $_ -match [regex]::Escape($asset) } | Select-Object -First 1
        if ($line) {
            $want = ($line.Trim() -split '\s+')[0]
            $got = (Get-FileHash -Path $zip -Algorithm SHA256).Hash
            if ($got -ne $want.ToUpperInvariant()) {
                throw "Checksum mismatch for ${asset}: expected $want, got $got. Refusing to install."
            }
            Write-Step 'Checksum verified.'
        }
    }

    Expand-Archive -Path $zip -DestinationPath $tmp -Force
    $staged = Join-Path $tmp $BinName
    if (-not (Test-Path $staged)) { throw "$asset did not contain $BinName." }

    # --- Install ------------------------------------------------------------
    # Windows locks a running .exe against overwrite but allows a rename, so an
    # upgrade while crew is open moves the old binary aside instead of failing.
    $dest = Join-Path $InstallDir $BinName
    if (Test-Path $dest) {
        $old = "$dest.old"
        Remove-Item $old -Force -ErrorAction SilentlyContinue
        try {
            Move-Item -Path $dest -Destination $old -Force
        } catch {
            throw "Could not replace $dest — is crew running? Close it and re-run."
        }
    }
    Move-Item -Path $staged -Destination $dest -Force
    Remove-Item "$dest.old" -Force -ErrorAction SilentlyContinue
    Write-Host ""
    Write-Host "Installed crew $Version to $dest"
} finally {
    Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

# --- PATH (user scope: HKCU\Environment, never HKLM) -------------------------

if (-not $NoPath) {
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $entries = @()
    if ($userPath) { $entries = $userPath -split ';' | Where-Object { $_ -ne '' } }
    $already = $entries | Where-Object { $_.TrimEnd('\') -ieq $InstallDir.TrimEnd('\') }
    if ($already) {
        Write-Step "$InstallDir is already on your PATH."
    } else {
        [Environment]::SetEnvironmentVariable('Path', (($entries + $InstallDir) -join ';'), 'User')
        # Also fix up THIS session, so the user can run crew without reopening.
        $env:Path = "$env:Path;$InstallDir"
        Write-Step "Added $InstallDir to your user PATH."
        Write-Note 'Open a new terminal for the PATH change to reach other shells.'
    }
}

# --- Start menu -------------------------------------------------------------
# Best-effort: an older binary without the subcommand must not fail the install.
try {
    & (Join-Path $InstallDir $BinName) install-app 2>$null | Out-Null
} catch {}

Write-Host ""
Write-Host "Run 'crew' to start."
Write-Host ""
Write-Host "Agents work with no API key at all if you are already signed in to"
Write-Host "claude, codex or opencode - crew finds them on PATH. Otherwise open"
Write-Host "the model picker with '/model' and add a provider key there."
Write-Host "Type '/update' inside crew to install new releases."
