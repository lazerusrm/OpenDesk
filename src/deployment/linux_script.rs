use crate::domain::server_config::ServerConfig;

pub struct LinuxDeploymentScriptInput<'a> {
    pub server_config: &'a ServerConfig,
    pub enrollment_token: &'a str,
    pub opendesk_base_url: &'a str,
}

pub fn render_linux_deployment_script(input: &LinuxDeploymentScriptInput<'_>) -> String {
    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail

OPENDESK_BASE_URL="{opendesk_base_url}"
ENROLLMENT_TOKEN="{enrollment_token}"
ID_SERVER="{id_server}"
RELAY_SERVER="{relay_server}"
API_SERVER="{api_server}"
PUBLIC_KEY="{public_key}"

if ! command -v rustdesk >/dev/null 2>&1; then
  echo "rustdesk client is required; install the official package first" >&2
  exit 1
fi

rustdesk --option custom-rendezvous-server "${{ID_SERVER}}"
rustdesk --option relay-server "${{RELAY_SERVER}}"
if [ -n "${{API_SERVER}}" ]; then
  rustdesk --option api-server "${{API_SERVER}}"
fi
if [ -n "${{PUBLIC_KEY}}" ]; then
  rustdesk --option key "${{PUBLIC_KEY}}"
fi

RUSTDESK_ID="$(rustdesk --get-id 2>/dev/null || true)"
HOSTNAME_VALUE="$(hostname)"
OS_FAMILY="linux"
ARCH_VALUE="$(uname -m)"

CHECKIN_HTTP_CODE="$(curl -fsS -o /dev/null -w '%{{http_code}}' -X POST "${{OPENDESK_BASE_URL}}/api/enrollments/check-in" \
  -H "Content-Type: application/json" \
  -d "{{\"enrollment_token\":\"${{ENROLLMENT_TOKEN}}\",\"rustdesk_id\":\"${{RUSTDESK_ID}}\",\"hostname\":\"${{HOSTNAME_VALUE}}\",\"os_family\":\"${{OS_FAMILY}}\",\"architecture\":\"${{ARCH_VALUE}}\"}}")"
echo "opendesk enrollment check-in http_status=${{CHECKIN_HTTP_CODE}}"
if [ "${{CHECKIN_HTTP_CODE}}" != "204" ]; then
  echo "opendesk enrollment check-in failed" >&2
  exit 1
fi
"#,
        opendesk_base_url = input.opendesk_base_url,
        enrollment_token = input.enrollment_token,
        id_server = input.server_config.id_server,
        relay_server = input.server_config.relay_server,
        api_server = input.server_config.api_server,
        public_key = input.server_config.public_key,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::server_config::default_server_config;

    #[test]
    fn render_linux_deployment_script_includes_server_values() {
        let config = default_server_config();
        let script = render_linux_deployment_script(&LinuxDeploymentScriptInput {
            server_config: &config,
            enrollment_token: "test-enrollment-token",
            opendesk_base_url: "https://rd-admin.example.com",
        });
        assert!(script.contains("custom-rendezvous-server"));
        assert!(script.contains("rd.example.com"));
        assert!(script.contains("/api/enrollments/check-in"));
        assert!(script.contains("opendesk enrollment check-in http_status="));
        assert!(script.contains("test-enrollment-token"));
    }
}