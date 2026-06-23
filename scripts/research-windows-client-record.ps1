param(
    [Parameter(Mandatory = $true)]
    [string]$RustDeskPath,

    [Parameter(Mandatory = $false)]
    [string]$ConfigString = "",

    [Parameter(Mandatory = $false)]
    [string]$DeployToken = "",

    [Parameter(Mandatory = $false)]
    [string]$CaseName = "windows-client"
)

$ErrorActionPreference = "Continue"

function New-SafeName {
    param([string]$Value)
    return (($Value -replace '[^A-Za-z0-9._-]+', '-') -replace '^-|-$', '')
}

function Invoke-Step {
    param(
        [string]$Name,
        [scriptblock]$Script
    )

    $started = Get-Date -Format o
    try {
        $output = & $Script 2>&1 | Out-String
        return [ordered]@{
            name = $Name
            started_at = $started
            exit_code = $LASTEXITCODE
            output = $output.Trim()
        }
    } catch {
        return [ordered]@{
            name = $Name
            started_at = $started
            exit_code = 1
            output = $_.Exception.Message
        }
    }
}

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
$manualDir = Join-Path $repoRoot "local/research/manual"
New-Item -ItemType Directory -Force -Path $manualDir | Out-Null

$stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMdd-HHmmss")
$safeCase = New-SafeName $CaseName
$jsonPath = Join-Path $manualDir "windows-client-$stamp-$safeCase.json"
$mdPath = Join-Path $manualDir "windows-client-$stamp-$safeCase.md"

$steps = New-Object System.Collections.Generic.List[object]
$steps.Add((Invoke-Step "version" { & $RustDeskPath --version }))
$steps.Add((Invoke-Step "get-id-before-config" { & $RustDeskPath --get-id }))

if ($ConfigString.Trim().Length -gt 0) {
    $steps.Add((Invoke-Step "apply-config" { & $RustDeskPath --config $ConfigString }))
    $steps.Add((Invoke-Step "get-id-after-config" { & $RustDeskPath --get-id }))
    $steps.Add((Invoke-Step "read-id-server" { & $RustDeskPath --option custom-rendezvous-server }))
    $steps.Add((Invoke-Step "read-relay-server" { & $RustDeskPath --option relay-server }))
    $steps.Add((Invoke-Step "read-api-server" { & $RustDeskPath --option api-server }))
    $steps.Add((Invoke-Step "read-key" { & $RustDeskPath --option key }))
}

$service = Get-Service -Name "RustDesk" -ErrorAction SilentlyContinue
if ($service) {
    $steps.Add((Invoke-Step "restart-service" { Restart-Service -Name "RustDesk" -Force; Get-Service -Name "RustDesk" | Format-List | Out-String }))
    $steps.Add((Invoke-Step "get-id-after-restart" { & $RustDeskPath --get-id }))
}

if ($DeployToken.Trim().Length -gt 0) {
    $steps.Add((Invoke-Step "deploy" { & $RustDeskPath --deploy --token $DeployToken }))
}

$record = [ordered]@{
    date = (Get-Date).ToUniversalTime().ToString("o")
    case_name = $CaseName
    endpoint_os = (Get-CimInstance Win32_OperatingSystem | Select-Object -ExpandProperty Caption)
    endpoint_version = (Get-CimInstance Win32_OperatingSystem | Select-Object -ExpandProperty Version)
    rustdesk_path = $RustDeskPath
    config_provided = $ConfigString.Trim().Length -gt 0
    deploy_token_provided = $DeployToken.Trim().Length -gt 0
    steps = $steps
}

$record | ConvertTo-Json -Depth 8 | Set-Content -Path $jsonPath -Encoding UTF8

@"
# Windows Client Validation Record

Date: $($record.date)
Case: $CaseName
Endpoint OS/version: $($record.endpoint_os) $($record.endpoint_version)
RustDesk path: $RustDeskPath
Raw artifact: $jsonPath
Final status: not-reviewed

## Review Checklist

- [ ] Installer signature/checksum was verified separately.
- [ ] Config values are redacted before public summary.
- [ ] Service/user context behavior is reviewed.
- [ ] Restart persistence is reviewed.
- [ ] Reinstall/upgrade persistence is reviewed if tested.
- [ ] Deploy endpoint behavior is reviewed if tested.
"@ | Set-Content -Path $mdPath -Encoding UTF8

Write-Output $mdPath
