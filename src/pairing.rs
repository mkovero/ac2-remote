use ac2_client::{KeyDir, RemoteAddr};
use ac2_discovery::Rig;
use ac2_zmq::PublicKey;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Connection {
    pub host: String,
    pub server_key: String,
}

impl Connection {
    pub fn from_rig(rig: &Rig) -> Result<Self, String> {
        if rig.advert.proto != ac2_proto::PROTO_VERSION {
            return Err(format!(
                "Protocol mismatch: rig uses {}, viewer uses {}",
                rig.advert.proto,
                ac2_proto::PROTO_VERSION
            ));
        }
        if rig.port == u16::MAX {
            return Err("Rig has no valid data port".into());
        }
        let key = PublicKey::from_z85(&rig.advert.server_key).map_err(|e| e.to_string())?;
        if key.fingerprint() != rig.advert.fingerprint {
            return Err("Rig advertisement's public key does not match its fingerprint".into());
        }
        let host = rig.connect_host();
        let host = if host.contains(':') {
            format!("[{host}]:{}", rig.port)
        } else {
            format!("{host}:{}", rig.port)
        };
        let connection = Self {
            host,
            server_key: key.to_z85(),
        };
        connection.validate()?;
        Ok(connection)
    }
    fn validate(&self) -> Result<(), String> {
        self.host.parse::<RemoteAddr>().map_err(|e| e.to_string())?;
        PublicKey::from_z85(&self.server_key).map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn trusted(&self, keys: &KeyDir) -> bool {
        let Ok(addr) = self.host.parse::<RemoteAddr>() else {
            return false;
        };
        let Ok(key) = PublicKey::from_z85(&self.server_key) else {
            return false;
        };
        if let Ok(pinned) = keys.server_key(&addr.host) {
            return pinned == key;
        }
        // A pinned key identifies the rig even after its DHCP address changes.
        keys.known_servers()
            .is_ok_and(|pins| pins.iter().any(|(_, pinned)| *pinned == key))
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        let parent = path.parent().ok_or("Connection file has no directory")?;
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let temporary = path.with_extension("tmp");
        std::fs::write(
            &temporary,
            serde_json::to_vec(self).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(temporary, path).map_err(|e| e.to_string())
    }
    pub fn load(path: &Path) -> Result<Option<Self>, String> {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.to_string()),
        };
        let connection: Self = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        connection.validate()?;
        Ok(Some(connection))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ac2_discovery::Advert;
    use std::net::IpAddr;

    fn rig(key: PublicKey) -> Rig {
        Rig {
            instance: "FOH._ac2._tcp.local.".into(),
            advert: Advert {
                name: "FOH".into(),
                version: "test".into(),
                proto: ac2_proto::PROTO_VERSION,
                fingerprint: key.fingerprint(),
                server_key: key.to_z85(),
            },
            host: "foh.local".into(),
            addresses: vec![IpAddr::from([10, 0, 0, 20])],
            port: 47820,
        }
    }

    #[test]
    fn discovery_prefills_but_never_trusts_or_replaces_a_pin() {
        let directory = tempfile::tempdir().unwrap();
        let keys = KeyDir::new(directory.path().join("keys"));
        let original = PublicKey::from_bytes([1; 32]);
        let connection = Connection::from_rig(&rig(original)).unwrap();
        assert_eq!(connection.host, "10.0.0.20:47820");
        assert!(!connection.trusted(&keys));
        keys.pin_server("10.0.0.20", original).unwrap();
        assert!(connection.trusted(&keys));
        let mut moved = rig(original);
        moved.addresses = vec![IpAddr::from([10, 0, 0, 21])];
        assert!(Connection::from_rig(&moved).unwrap().trusted(&keys));
        let replacement = Connection::from_rig(&rig(PublicKey::from_bytes([2; 32]))).unwrap();
        assert!(!replacement.trusted(&keys));
        assert_eq!(keys.server_key("10.0.0.20").unwrap(), original);
        let file = directory.path().join("connection.json");
        connection.save(&file).unwrap();
        let loaded = Connection::load(&file).unwrap().unwrap();
        assert_eq!(connection, loaded);
        assert!(loaded.trusted(&keys));
        assert!(!loaded.trusted(&KeyDir::new(directory.path().join("different-keys"))));
    }

    #[test]
    fn inconsistent_and_incompatible_rigs_are_rejected() {
        let mut rig = rig(PublicKey::from_bytes([3; 32]));
        rig.advert.fingerprint = PublicKey::from_bytes([4; 32]).fingerprint();
        assert!(
            Connection::from_rig(&rig)
                .unwrap_err()
                .contains("fingerprint")
        );
        rig.advert.fingerprint = PublicKey::from_bytes([3; 32]).fingerprint();
        rig.advert.proto += 1;
        assert!(
            Connection::from_rig(&rig)
                .unwrap_err()
                .contains("Protocol mismatch")
        );
        rig.advert.proto = ac2_proto::PROTO_VERSION;
        rig.port = u16::MAX;
        assert!(Connection::from_rig(&rig).is_err());
    }

    #[test]
    fn ipv6_address_and_custom_port_survive_restart() {
        let mut rig = rig(PublicKey::from_bytes([5; 32]));
        rig.addresses = vec!["2001:db8::5".parse().unwrap()];
        rig.port = 6000;
        let connection = Connection::from_rig(&rig).unwrap();
        assert_eq!(connection.host, "[2001:db8::5]:6000");
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("connection.json");
        assert_eq!(Connection::load(&file).unwrap(), None);
        connection.save(&file).unwrap();
        assert_eq!(Connection::load(&file).unwrap(), Some(connection));
        std::fs::write(&file, "{broken").unwrap();
        assert!(Connection::load(&file).is_err());
    }
}
