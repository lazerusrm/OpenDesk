use crate::domain::server_config::ServerConfig;

pub fn generate_filename_custom_server(config: &ServerConfig) -> String {
    let host = config.id_server.trim();
    let relay = config.relay_server.trim();
    let key = config.public_key.trim();
    let api = config.api_server.trim();
    if !api.is_empty() && !key.is_empty() {
        format!("rustdesk-host={host},api={api},key={key},relay={relay}.exe")
    } else if !key.is_empty() && !relay.is_empty() {
        format!("rustdesk-host={host},key={key},relay={relay}.exe")
    } else {
        format!("rustdesk-host={host}.exe")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::server_config::default_server_config;

    #[test]
    fn filename_includes_host_key_and_relay() {
        let mut config = default_server_config();
        config.public_key = "PUBLICKEY".to_string();
        let name = generate_filename_custom_server(&config);
        assert!(name.starts_with("rustdesk-host=rd.example.com,"));
        assert!(name.contains("key=PUBLICKEY"));
        assert!(name.contains("relay=rd.example.com"));
        assert!(name.ends_with(".exe"));
    }

    #[test]
    fn filename_includes_api_when_configured() {
        let mut config = default_server_config();
        config.public_key = "PUBLICKEY".to_string();
        config.api_server = "https://api.example.net".to_string();
        let name = generate_filename_custom_server(&config);
        assert!(name.contains("api=https://api.example.net"));
    }
}