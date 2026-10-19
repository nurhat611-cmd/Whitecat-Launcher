param(
    [Parameter(Mandatory = $true)]
    [string]$InstallDir
)

$ErrorActionPreference = "Stop"
$releaseUrl = "https://github.com/nurhat611-cmd/Whitecat-Launcher/releases/latest/download/launcher.zip"
$proxyUrl = "https://ghproxy.com/$releaseUrl"
$tempDir = Join-Path $env:TEMP ("WhitecatLauncher-" + [guid]::NewGuid().ToString("N"))
$archivePath = Join-Path $tempDir "launcher.zip"
$extractDir = Join-Path $tempDir "payload"

try {
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
    New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

    $sourceExe = $null
    foreach ($url in @($releaseUrl, $proxyUrl)) {
        try {
            Write-Output "Скачивание MyLauncher..."
            Invoke-WebRequest -Uri $url -OutFile $archivePath -UseBasicParsing -TimeoutSec 120
            if (-not (Test-Path $archivePath) -or (Get-Item $archivePath).Length -lt 100KB) {
                throw "Сервер вернул пустой или неполный архив."
            }

            Expand-Archive -LiteralPath $archivePath -DestinationPath $extractDir -Force
            $candidate = Join-Path $extractDir "MyLauncher.exe"
            if (-not (Test-Path $candidate -PathType Leaf)) {
                throw "В архиве отсутствует MyLauncher.exe."
            }
            if ((Get-Item $candidate).Length -lt 1MB) {
                throw "Загруженный MyLauncher.exe выглядит повреждённым."
            }

            $sourceExe = $candidate
            break
        }
        catch {
            Remove-Item $archivePath -Force -ErrorAction SilentlyContinue
            Remove-Item $extractDir -Recurse -Force -ErrorAction SilentlyContinue
            Write-Output "Основная ссылка недоступна; проверяем резервный источник..."
        }
    }

    if (-not $sourceExe) {
        throw "Не удалось скачать и проверить launcher.zip ни с GitHub Releases, ни через ghproxy.com."
    }

    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Copy-Item -LiteralPath $sourceExe -Destination (Join-Path $InstallDir "MyLauncher.exe") -Force
    Write-Output "MyLauncher установлен в $InstallDir"
    exit 0
}
catch {
    Write-Error $_.Exception.Message
    exit 1
}
finally {
    Remove-Item $tempDir -Recurse -Force -ErrorAction SilentlyContinue
}
