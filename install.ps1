$ErrorActionPreference = 'Stop'

$repo = 'DranxX/CodeMap'
$installDir = if ($env:CODEMAP_INSTALL_DIR) {
  $env:CODEMAP_INSTALL_DIR
} elseif ($env:LOCALAPPDATA) {
  Join-Path $env:LOCALAPPDATA 'Programs\CodeMap'
} else {
  Join-Path $HOME 'AppData\Local\Programs\CodeMap'
}
$version = $env:CODEMAP_VERSION

$architecture = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
$asset = switch ($architecture) {
  'AMD64' { 'codemap-windows-x86_64.exe' }
  'ARM64' { 'codemap-windows-aarch64.exe' }
  default { throw "Unsupported architecture: $architecture. CodeMap provides Windows x86-64 and ARM64 binaries." }
}

$baseUrl = "https://github.com/$repo/releases"
$baseUrl = if ($version) { "$baseUrl/download/$version" } else { "$baseUrl/latest/download" }
$downloadUrl = "$baseUrl/$asset"
$targetPath = Join-Path $installDir 'codemap.exe'
$tempPath = Join-Path $installDir ('.codemap-' + [guid]::NewGuid().ToString('N') + '.exe')
$displayVersion = if ($version) { $version } else { 'latest' }

New-Item -ItemType Directory -Force -Path $installDir | Out-Null
try {
  Write-Host "Downloading CodeMap $displayVersion for windows/$architecture..."
  Invoke-WebRequest -Uri $downloadUrl -OutFile $tempPath -UseBasicParsing
  Move-Item -Path $tempPath -Destination $targetPath -Force
} catch {
  throw "Could not install CodeMap from $downloadUrl. Confirm the matching release asset exists. $($_.Exception.Message)"
} finally {
  Remove-Item -Path $tempPath -Force -ErrorAction SilentlyContinue
}

$normalizedInstallDir = [System.IO.Path]::GetFullPath($installDir).TrimEnd('\')
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
$pathEntries = @($userPath -split ';' | Where-Object { $_ } | ForEach-Object { $_.TrimEnd('\') })
if ($pathEntries -notcontains $normalizedInstallDir) {
  [Environment]::SetEnvironmentVariable('Path', (@($normalizedInstallDir) + $pathEntries) -join ';', 'User')
}
if ((@($env:Path -split ';' | ForEach-Object { $_.TrimEnd('\') })) -notcontains $normalizedInstallDir) {
  $env:Path = "$normalizedInstallDir;$env:Path"
}

Write-Host "Installed CodeMap to $targetPath"
Write-Host "Run 'codemap' inside a repository, or 'codemap C:\path\to\repository'."
