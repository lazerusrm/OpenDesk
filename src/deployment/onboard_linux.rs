use crate::domain::server_config::ServerConfig;

fn shell_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

pub struct OnboardLinuxScriptInput<'a> {
    pub server_config: &'a ServerConfig,
    pub enrollment_token: &'a str,
    pub opendesk_base_url: &'a str,
    pub linux_package_url: Option<&'a str>,
}

pub fn render_onboard_linux_script(input: &OnboardLinuxScriptInput<'_>) -> String {
    format!(
        r#"#!/usr/bin/env bash
set -euo pipefail

OPENDESK_BASE_URL={opendesk_base_url}
ENROLLMENT_TOKEN={enrollment_token}
ID_SERVER={id_server}
RELAY_SERVER={relay_server}
API_SERVER={api_server}
PUBLIC_KEY={public_key}
LINUX_PACKAGE_URL={linux_package_url}

if ! command -v rustdesk >/dev/null 2>&1; then
  if [ -z "${{LINUX_PACKAGE_URL}}" ]; then
    echo "rustdesk client is required; install the official package first" >&2
    exit 1
  fi
  tmp="$(mktemp)"
  curl -fsSL "${{LINUX_PACKAGE_URL}}" -o "${{tmp}}"
  if command -v apt-get >/dev/null 2>&1; then
    apt-get install -y "${{tmp}}"
  elif command -v dnf >/dev/null 2>&1; then
    dnf install -y "${{tmp}}"
  else
    echo "cannot install the official rustdesk package on this distribution" >&2
    exit 1
  fi
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
CHECKIN_HTTP_CODE="$(curl -fsS -o /dev/null -w '%{{http_code}}' -X POST "${{OPENDESK_BASE_URL}}/api/enrollments/check-in" \
  -H "Content-Type: application/json" \
  -d "{{\"enrollment_token\":\"${{ENROLLMENT_TOKEN}}\",\"rustdesk_id\":\"${{RUSTDESK_ID}}\",\"hostname\":\"${{HOSTNAME_VALUE}}\",\"os_family\":\"linux\",\"architecture\":\"$(uname -m)\"}}")"
echo "opendesk enrollment check-in http_status=${{CHECKIN_HTTP_CODE}}"
if [ "${{CHECKIN_HTTP_CODE}}" != "204" ]; then
  echo "opendesk enrollment check-in failed" >&2
  exit 1
fi
"#,
        opendesk_base_url = shell_literal(input.opendesk_base_url),
        enrollment_token = shell_literal(input.enrollment_token),
        id_server = shell_literal(&input.server_config.id_server),
        relay_server = shell_literal(&input.server_config.relay_server),
        api_server = shell_literal(&input.server_config.api_server),
        public_key = shell_literal(&input.server_config.public_key),
        linux_package_url = shell_literal(input.linux_package_url.unwrap_or("")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::server_config::default_server_config;

    #[test]
    fn render_onboard_linux_script_escapes_values() {
        let mut config = default_server_config();
        config.id_server = "rd.example.com'; touch /tmp/x".to_string();
        let script = render_onboard_linux_script(&OnboardLinuxScriptInput {
            server_config: &config,
            enrollment_token: "token$(unsafe)",
            opendesk_base_url: "https://rd.example.com",
            linux_package_url: Some("https://github.com/rustdesk/rustdesk/releases/download/x.deb"),
        });
        assert!(script.contains("ID_SERVER='rd.example.com'\"'\"'; touch /tmp/x'"));
        assert!(script.contains("ENROLLMENT_TOKEN='token$(unsafe)'"));
        assert!(script.contains("/api/enrollments/check-in"));
    }
}
