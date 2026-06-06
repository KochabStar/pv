param(
    [string] $Repo = "OWNER/pv",
    [string] $Version = "latest",
    [string] $InstallDir = (Join-Path $env:USERPROFILE ".pv"),
    [string] $AssetPattern = "windows|pc-windows-msvc|x86_64",
    [string] $DownloadUrl = "",
    [string] $MainBucketUrl = "",
    [string] $MainBucketGitUrl = "local",
    [switch] $NoPathUpdate
)

$ErrorActionPreference = "Stop"
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

function Write-Status {
    param([string] $Message)
    Write-Host "pv: $Message"
}

function New-Directory {
    param([string] $Path)
    if (-not (Test-Path -LiteralPath $Path)) {
        New-Item -ItemType Directory -Force -Path $Path | Out-Null
    }
}

function Invoke-DownloadFile {
    param(
        [string] $Url,
        [string] $OutFile
    )
    $localPath = Resolve-LocalFilePath -Value $Url
    if ($localPath) {
        Write-Status "using local file $localPath"
        Copy-Item -LiteralPath $localPath -Destination $OutFile -Force
        return
    }

    Write-Status "downloading $Url"
    Invoke-WebRequest -Uri $Url -OutFile $OutFile -UseBasicParsing
}

function Resolve-LocalFilePath {
    param([string] $Value)

    if (Test-Path -LiteralPath $Value) {
        return [IO.Path]::GetFullPath($Value)
    }

    try {
        $uri = [Uri] $Value
        if ($uri.IsFile) {
            return $uri.LocalPath
        }
    } catch {
        return $null
    }

    $null
}

function Get-GitHubRelease {
    param(
        [string] $Repo,
        [string] $Version
    )

    if ($Repo -eq "OWNER/pv") {
        throw "Set -Repo to your GitHub repository, for example: -Repo owner/pv"
    }

    if ($Version -eq "latest") {
        $api = "https://api.github.com/repos/$Repo/releases/latest"
    } else {
        $api = "https://api.github.com/repos/$Repo/releases/tags/$Version"
    }

    Write-Status "resolving release $Repo@$Version"
    Invoke-RestMethod -Uri $api -UseBasicParsing
}

function Select-ReleaseAsset {
    param(
        [object] $Release,
        [string] $Pattern
    )

    $zipAssets = @($Release.assets | Where-Object { $_.name -match '\.zip$' })
    $matches = @($zipAssets | Where-Object { $_.name -match $Pattern })
    if ($matches.Count -eq 0 -and $zipAssets.Count -eq 1) {
        return $zipAssets[0]
    }
    if ($matches.Count -eq 0) {
        $names = ($zipAssets | ForEach-Object { $_.name }) -join ", "
        throw "No release zip matched '$Pattern'. Available zip assets: $names"
    }
    if ($matches.Count -gt 1) {
        $names = ($matches | ForEach-Object { $_.name }) -join ", "
        throw "More than one release zip matched '$Pattern': $names"
    }
    $matches[0]
}

function Expand-Zip {
    param([string] $ZipPath)

    $destination = Join-Path ([IO.Path]::GetTempPath()) "pv-install-$([Guid]::NewGuid())"
    New-Directory $destination
    Expand-Archive -LiteralPath $ZipPath -DestinationPath $destination -Force
    $destination
}

function Find-File {
    param(
        [string] $Root,
        [string] $Name
    )

    Get-ChildItem -LiteralPath $Root -Recurse -File -Filter $Name |
        Select-Object -First 1
}

function Find-MainBucketDirectory {
    param([string] $Root)

    $bucket = Get-ChildItem -LiteralPath $Root -Recurse -Directory |
        Where-Object { $_.Name -eq "main-bucket" } |
        Select-Object -First 1
    if ($bucket) {
        return $bucket
    }

    Get-ChildItem -LiteralPath $Root -Recurse -Directory |
        Where-Object { $_.FullName -like "*\buckets\main" } |
        Select-Object -First 1
}

function Copy-DirectoryContents {
    param(
        [string] $Source,
        [string] $Destination
    )

    New-Directory $Destination
    Get-ChildItem -LiteralPath $Source -Force |
        Copy-Item -Destination $Destination -Recurse -Force
}

function Install-PvBinaries {
    param(
        [string] $ExtractedRelease,
        [string] $BinDir
    )

    New-Directory $BinDir
    $pvExe = Find-File -Root $ExtractedRelease -Name "pv.exe"
    $shimExe = Find-File -Root $ExtractedRelease -Name "pv-shim.exe"
    if (-not $pvExe) {
        throw "Release archive does not contain pv.exe"
    }
    if (-not $shimExe) {
        throw "Release archive does not contain pv-shim.exe"
    }

    Copy-Item -LiteralPath $pvExe.FullName -Destination (Join-Path $BinDir "pv.exe") -Force
    Copy-Item -LiteralPath $shimExe.FullName -Destination (Join-Path $BinDir "pv-shim.exe") -Force
}

function Install-MainBucket {
    param(
        [string] $InstallDir,
        [string] $ExtractedRelease,
        [string] $MainBucketUrl
    )

    $bucketDir = Join-Path $InstallDir "buckets\main"
    if (Test-Path -LiteralPath $bucketDir) {
        $existing = Get-ChildItem -LiteralPath $bucketDir -Force -ErrorAction SilentlyContinue |
            Select-Object -First 1
        if ($existing) {
            Write-Status "main bucket already exists, skipping"
            return
        }
    }

    if ($MainBucketUrl) {
        $bucketZip = Join-Path ([IO.Path]::GetTempPath()) "pv-main-bucket-$([Guid]::NewGuid()).zip"
        Invoke-DownloadFile -Url $MainBucketUrl -OutFile $bucketZip
        $bucketExtract = Expand-Zip -ZipPath $bucketZip
        $bucket = Find-MainBucketDirectory -Root $bucketExtract
        if (-not $bucket) {
            $bucket = Get-ChildItem -LiteralPath $bucketExtract -Directory | Select-Object -First 1
        }
        if (-not $bucket) {
            throw "Main bucket archive does not contain a bucket directory"
        }
        Copy-DirectoryContents -Source $bucket.FullName -Destination $bucketDir
        return
    }

    $releaseBucket = Find-MainBucketDirectory -Root $ExtractedRelease
    if ($releaseBucket) {
        Copy-DirectoryContents -Source $releaseBucket.FullName -Destination $bucketDir
        return
    }

    Write-Status "no main bucket snapshot found; pass -MainBucketUrl to enable first-run package installs"
    New-Directory $bucketDir
}

function Write-PvConfig {
    param(
        [string] $InstallDir,
        [string] $MainBucketGitUrl
    )

    $configFile = Join-Path $InstallDir "config.toml"
    if (Test-Path -LiteralPath $configFile) {
        return
    }

    @"
[[buckets]]
name = "main"
url = "$MainBucketGitUrl"
"@ | Set-Content -LiteralPath $configFile -Encoding UTF8
}

function Set-UserPath {
    param([string[]] $Dirs)

    $current = [Environment]::GetEnvironmentVariable("Path", "User")
    $entries = @()
    if ($current) {
        $entries = @($current -split ";" | Where-Object { $_ })
    }

    for ($index = $Dirs.Count - 1; $index -ge 0; $index--) {
        $dir = $Dirs[$index]
        $exists = $false
        foreach ($entry in $entries) {
            if ($entry -ieq $dir) {
                $exists = $true
                break
            }
        }
        if (-not $exists) {
            $entries = @($dir) + $entries
        }
    }

    $updated = ($entries -join ";")
    [Environment]::SetEnvironmentVariable("Path", $updated, "User")

    $machine = [Environment]::GetEnvironmentVariable("Path", "Machine")
    if ($machine) {
        $env:PATH = "$updated;$machine"
    } else {
        $env:PATH = $updated
    }
}

function Install-Pv {
    if (-not $env:USERPROFILE) {
        throw "USERPROFILE is not set"
    }
    if (-not [Environment]::Is64BitOperatingSystem) {
        throw "pv currently expects a 64-bit Windows release asset"
    }

    $release = $null
    $assetUrl = $DownloadUrl
    if (-not $assetUrl) {
        $release = Get-GitHubRelease -Repo $Repo -Version $Version
        $asset = Select-ReleaseAsset -Release $release -Pattern $AssetPattern
        $assetUrl = $asset.browser_download_url
        if (-not $MainBucketUrl) {
            $bucketAsset = @($release.assets | Where-Object { $_.name -match '^main-bucket.*\.zip$' }) |
                Select-Object -First 1
            if ($bucketAsset) {
                $script:MainBucketUrl = $bucketAsset.browser_download_url
            }
        }
    }

    $root = [IO.Path]::GetFullPath($InstallDir)
    $binDir = Join-Path $root "bin"
    $shimsDir = Join-Path $root "shims"
    New-Directory $root
    New-Directory $binDir
    New-Directory $shimsDir

    $releaseZip = Join-Path ([IO.Path]::GetTempPath()) "pv-release-$([Guid]::NewGuid()).zip"
    Invoke-DownloadFile -Url $assetUrl -OutFile $releaseZip
    $extractedRelease = Expand-Zip -ZipPath $releaseZip

    Install-PvBinaries -ExtractedRelease $extractedRelease -BinDir $binDir
    Install-MainBucket -InstallDir $root -ExtractedRelease $extractedRelease -MainBucketUrl $script:MainBucketUrl
    Write-PvConfig -InstallDir $root -MainBucketGitUrl $MainBucketGitUrl

    if (-not $NoPathUpdate) {
        Set-UserPath -Dirs @($binDir, $shimsDir)
        Write-Status "added $binDir and $shimsDir to the user PATH"
    }

    Write-Status "installed pv to $root"
    Write-Status "open a new terminal and run: pv --version"
}

Install-Pv
