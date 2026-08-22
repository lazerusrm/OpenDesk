use crate::domain::server_config::ServerConfig;

fn powershell_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

pub struct OnboardWindowsScriptInput<'a> {
    pub server_config: &'a ServerConfig,
    pub enrollment_token: &'a str,
    pub opendesk_base_url: &'a str,
    pub setup_url: &'a str,
}

pub fn render_onboard_windows_script(input: &OnboardWindowsScriptInput<'_>) -> String {
    format!(
        r#"#requires -Version 5.1
$ErrorActionPreference = "Stop"

$OpenDeskBaseUrl = {opendesk_base_url}
$EnrollmentToken = {enrollment_token}
$SetupUrl = {setup_url}
$IdServer = {id_server}
$RelayServer = {relay_server}
$ApiServer = {api_server}
$PublicKey = {public_key}

$tmp = Join-Path $env:TEMP "opendesk-windows-setup.exe"
try {{
    Invoke-WebRequest -UseBasicParsing -Uri $SetupUrl -OutFile $tmp
    Start-Process -FilePath $tmp -Wait
}} catch {{
    if (-not (Get-Command rustdesk -ErrorAction SilentlyContinue)) {{
        throw "Windows installer download failed and rustdesk is not installed"
    }}
}}

$RustDeskCmd = Get-Command rustdesk -ErrorAction SilentlyContinue
if ($RustDeskCmd) {{
    & rustdesk --option custom-rendezvous-server $IdServer
    & rustdesk --option relay-server $RelayServer
    if ($ApiServer) {{
        & rustdesk --option api-server $ApiServer
    }}
    if ($PublicKey) {{
        & rustdesk --option key $PublicKey
    }}
}}

$RustDeskId = ""
try {{ $RustDeskId = (& rustdesk --get-id 2>$null) }} catch {{}}
if (-not $RustDeskId) {{ $RustDeskId = "" }}
$Body = @{{
    enrollment_token = $EnrollmentToken
    rustdesk_id = "$RustDeskId"
    hostname = $env:COMPUTERNAME
    os_family = "windows"
    architecture = $env:PROCESSOR_ARCHITECTURE
}} | ConvertTo-Json -Compress
Invoke-RestMethod -Method Post -Uri "$OpenDeskBaseUrl/api/enrollments/check-in" -ContentType "application/json" -Body $Body
"#,
        opendesk_base_url = powershell_literal(input.opendesk_base_url),
        enrollment_token = powershell_literal(input.enrollment_token),
        setup_url = powershell_literal(input.setup_url),
        id_server = powershell_literal(&input.server_config.id_server),
        relay_server = powershell_literal(&input.server_config.relay_server),
        api_server = powershell_literal(&input.server_config.api_server),
        public_key = powershell_literal(&input.server_config.public_key),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::server_config::default_server_config;

    #[test]
    fn render_onboard_windows_script_escapes_values() {
        let mut config = default_server_config();
        config.relay_server = "relay.example.com'; Remove-Item C:\\".to_string();
        let script = render_onboard_windows_script(&OnboardWindowsScriptInput {
            server_config: &config,
            enrollment_token: "token$(unsafe)",
            opendesk_base_url: "https://rd.example.com",
            setup_url: "https://rd.example.com/onboard/token/windows/setup.exe",
        });
        assert!(script.contains("$RelayServer = 'relay.example.com''; Remove-Item C:\\'"));
        assert!(script.contains("$EnrollmentToken = 'token$(unsafe)'"));
        assert!(script.contains("/api/enrollments/check-in"));
    }
}
