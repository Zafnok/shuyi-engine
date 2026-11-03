# A real render endpoint on a hosted Windows runner, which has none (ticket 3031,
# ADR-0067): Scream, a virtual sound card, pinned by its hash. It runs in CI only and
# nothing of it ships, as PulseAudio on Linux.
#
# Its signature has no timestamp and its certificate ended in 2023, so Windows waits
# for a person who isn't there to trust it. The clock goes inside the certificate's
# dates for the install alone, and comes back. A runner has Secure Boot off, so the
# kernel loads the driver.
$ErrorActionPreference = 'Stop'
$version = '4.0'
$hash = 'FA33E25F9A46C61E4E0CD83362C51C3D2A45C6FE4091AAD7507E240E40F1A520'
$zip = Join-Path $env:RUNNER_TEMP 'scream.zip'
$out = Join-Path $env:RUNNER_TEMP 'scream'

Invoke-WebRequest "https://github.com/duncanthrax/scream/releases/download/$version/Scream$version.zip" -OutFile $zip
if ((Get-FileHash $zip -Algorithm SHA256).Hash -ne $hash) { throw "Scream $version isn't the file pinned here" }
Expand-Archive $zip -DestinationPath $out -Force
$driver = Join-Path $out 'Install\driver\x64'
$devcon = Join-Path $out 'Install\helpers\devcon-x64.exe'

Start-Service audiosrv
$certificate = (Get-AuthenticodeSignature (Join-Path $driver 'scream.cat')).SignerCertificate
$store = [System.Security.Cryptography.X509Certificates.X509Store]::new('TrustedPublisher', 'LocalMachine')
$store.Open('ReadWrite')
$store.Add($certificate)
$store.Close()

# Both of what set the clock: Windows' own and the virtual machine's, which can set it
# back while the install reads the certificate's dates.
$timekeepers = 'w32time', 'vmictimesync'
$timekeepers | ForEach-Object { Stop-Service $_ -Force -ErrorAction SilentlyContinue }
$now = Get-Date
$clock = [System.Diagnostics.Stopwatch]::StartNew()
$exit = $null
try {
    foreach ($attempt in 1, 2) {
        Set-Date -Date ([datetime]'2023-01-15T12:00:00') | Out-Null
        if ((Get-Date).Year -ne 2023) { throw 'the clock would not stay inside the certificate''s dates' }
        $process = Start-Process -FilePath $devcon -ArgumentList 'install', 'Scream.inf', '*Scream' -WorkingDirectory $driver -PassThru -NoNewWindow
        # A prompt nobody can answer would otherwise hold the job for hours.
        if ($process.WaitForExit(60000)) {
            $exit = $process.ExitCode
            break
        }
        Stop-Process -Id $process.Id -Force
        Write-Output "attempt $attempt of the driver install waited for a person; the clock read $(Get-Date)"
    }
} finally {
    Set-Date -Date $now.Add($clock.Elapsed) | Out-Null
    $timekeepers | ForEach-Object { Start-Service $_ -ErrorAction SilentlyContinue }
}
if ($null -eq $exit) { throw 'the driver install is waiting for a person' }
if ($exit -ne 0) { throw "the driver install exited with $exit" }

Restart-Service audiosrv -Force
$deadline = [DateTime]::UtcNow.AddSeconds(30)
while (-not (Get-PnpDevice -Class AudioEndpoint -Status OK -ErrorAction SilentlyContinue)) {
    if ([DateTime]::UtcNow -gt $deadline) { throw 'no render endpoint came of the driver' }
    Start-Sleep 1
}
Get-PnpDevice -Class AudioEndpoint | Format-Table FriendlyName, Status -AutoSize
