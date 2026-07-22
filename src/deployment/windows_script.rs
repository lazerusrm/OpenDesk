use crate::domain::server_config::ServerConfig;

fn powershell_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

pub struct WindowsDeploymentScriptInput<'a> {
    pub server_config: &'a ServerConfig,
    pub enrollment_token: &'a str,
    pub opendesk_base_url: &'a str,
}

pub fn render_windows_deployment_script(input: &WindowsDeploymentScriptInput<'_>) -> String {
    format!(
        r#"#requires -Version 5.1
$ErrorActionPreference = "Stop"

$OpenDeskBaseUrl = {opendesk_base_url}
$EnrollmentToken = {enrollment_token}
$IdServer = {id_server}
$RelayServer = {relay_server}
$ApiServer = {api_server}
$PublicKey = {public_key}

$RustDeskCmd = Get-Command rustdesk -ErrorAction SilentlyContinue
if (-not $RustDeskCmd) {{
    throw "rustdesk client is required; install the official package first"
}}

& rustdesk --option custom-rendezvous-server $IdServer
& rustdesk --option relay-server $RelayServer
if ($ApiServer) {{
    & rustdesk --option api-server $ApiServer
}}
if ($PublicKey) {{
    & rustdesk --option key $PublicKey
}}

$RustDeskId = (& rustdesk --get-id 2>$null)
if (-not $RustDeskId) {{ $RustDeskId = "" }}
$HostnameValue = $env:COMPUTERNAME
$OsFamily = "windows"
$ArchValue = $env:PROCESSOR_ARCHITECTURE

$Body = @{{
    enrollment_token = $EnrollmentToken
    rustdesk_id = $RustDeskId
    hostname = $HostnameValue
    os_family = $OsFamily
    architecture = $ArchValue
}} | ConvertTo-Json -Compress

Invoke-RestMethod -Method Post -Uri "$OpenDeskBaseUrl/api/enrollments/check-in" -ContentType "application/json" -Body $Body
"#,
        opendesk_base_url = powershell_literal(input.opendesk_base_url),
        enrollment_token = powershell_literal(input.enrollment_token),
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
    fn render_windows_deployment_script_escapes_powershell_metacharacters() {
        let mut config = default_server_config();
        config.id_server = "rd.example.com'; Remove-Item C:\\ -Force".to_string();
        let script = render_windows_deployment_script(&WindowsDeploymentScriptInput {
            server_config: &config,
            enrollment_token: "token$(unsafe)",
            opendesk_base_url: "https://rd-admin.example.com",
        });
        assert!(script.contains("$IdServer = 'rd.example.com''; Remove-Item C:\\ -Force'"));
        assert!(script.contains("$EnrollmentToken = 'token$(unsafe)'"));
    }
}
