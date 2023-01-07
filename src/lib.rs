#[cfg(not(any(feature = "online", feature = "offline")))]
compile_error!("please select at least one feature (uuid-mc)");

use thiserror::Error;
use uuid::{Uuid, Version};

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid uuid")]
    InvalidUuid,

    #[error("invalid username")]
    InvalidUsername,

    #[cfg(feature = "online")]
    #[error("ureq transport error: {0}")]
    Transport(ureq::Transport),

    #[error("unknown")]
    Unknown,
}

type Result<T> = std::result::Result<T, Error>;

#[cfg(feature = "online")]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct OnlineUuid(Uuid);

#[cfg(feature = "offline")]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct OfflineUuid(Uuid);

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum PlayerUuid {
    #[cfg(feature = "online")]
    Online(OnlineUuid),
    #[cfg(feature = "offline")]
    Offline(OfflineUuid),
}

#[cfg(feature = "online")]
#[derive(serde::Deserialize)]
struct OnlineUuidResponse {
    name: String,
    id: Uuid,
}

#[cfg(feature = "online")]
impl OnlineUuid {
    pub fn get_username(&self) -> Result<String> {
        let response = ureq::get(&format!(
            "https://sessionserver.mojang.com/session/minecraft/profile/{}",
            self.0
        ))
        .call();

        match response {
            Ok(data) => {
                let response: OnlineUuidResponse = data.into_json().map_err(|_| Error::Unknown)?;
                Ok(response.name)
            }
            Err(ureq::Error::Status(_, _)) => Err(Error::InvalidUsername),
            Err(ureq::Error::Transport(x)) => Err(Error::Transport(x)),
        }
    }
}

impl PlayerUuid {
    #[cfg(feature = "online")]
    pub fn new_with_online_username(username: &str) -> Result<Self> {
        let response = ureq::get(&format!(
            "https://api.mojang.com/users/profiles/minecraft/{}",
            username
        ))
        .call();

        match response {
            Ok(data) => {
                let response: OnlineUuidResponse = data.into_json().map_err(|_| Error::Unknown)?;
                Ok(Self::Online(OnlineUuid(response.id)))
            }
            Err(ureq::Error::Status(_, _)) => Err(Error::InvalidUsername),
            Err(ureq::Error::Transport(x)) => Err(Error::Transport(x)),
        }
    }

    #[cfg(feature = "offline")]
    pub fn new_with_offline_username(username: &str) -> Self {
        let mut hash = md5::compute(format!("OfflinePlayer:{}", username)).0;
        hash[6] = hash[6] & 0x0f | 0x30; // uuid version 3
        hash[8] = hash[8] & 0x3f | 0x80; // RFC4122 variant

        let uuid = Uuid::from_bytes(hash);
        Self::Offline(OfflineUuid(uuid))
    }

    pub fn new_with_uuid(uuid: Uuid) -> Result<Self> {
        match uuid.get_version() {
            #[cfg(feature = "online")]
            Some(Version::Random) => Ok(Self::Online(OnlineUuid(uuid))),
            #[cfg(feature = "offline")]
            Some(Version::Md5) => Ok(Self::Offline(OfflineUuid(uuid))),
            _ => Err(Error::InvalidUuid),
        }
    }

    pub fn as_uuid(&self) -> &Uuid {
        match self {
            #[cfg(feature = "online")]
            Self::Online(OnlineUuid(uuid)) => uuid,
            #[cfg(feature = "offline")]
            Self::Offline(OfflineUuid(uuid)) => uuid,
        }
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        self.as_uuid().as_bytes()
    }

    #[cfg(feature = "offline")]
    pub fn unwrap_offline(self) -> OfflineUuid {
        match self {
            #[cfg(feature = "online")]
            Self::Online(_) => panic!("unwrap_offline called on an online uuid"),
            Self::Offline(uuid) => uuid,
        }
    }

    #[cfg(feature = "online")]
    pub fn unwrap_online(self) -> OnlineUuid {
        match self {
            Self::Online(uuid) => uuid,
            #[cfg(feature = "offline")]
            Self::Offline(_) => panic!("unwrap_online called on an offline uuid"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "offline")]
    #[test]
    fn offline_uuids() {
        let values = vec![
            ("boolean_coercion", "db62bdfb-eddc-3acc-a14e-c703aba52549"),
            ("BooleanCoercion", "44d050b1-46c8-37a8-b511-7023ae304192"),
            ("bool", "e9fd750e-29c2-3d85-80c9-64618059d454"),
            ("BOOL", "c5d06acf-0ef6-3a68-bf0b-b57806bcbef5"),
            ("BoOl", "e38f2cf4-72d2-3a84-8278-fed6908d2746"),
            ("booleancoercion", "072a2e03-56ce-3960-9391-c56afe17e317"),
        ];

        values
            .into_iter()
            .map(|(username, uuid)| {
                (
                    *PlayerUuid::new_with_offline_username(username).as_uuid(),
                    Uuid::try_parse(uuid).unwrap(),
                )
            })
            .for_each(|(uuid1, uuid2)| assert_eq!(uuid1, uuid2));
    }

    #[cfg(feature = "online")]
    #[test]
    fn online_uuids() {
        let values = vec![
            ("Notch", "069a79f4-44e9-4726-a5be-fca90e38aaf5"),
            ("dinnerbone", "61699b2e-d327-4a01-9f1e-0ea8c3f06bc6"),
            ("Dinnerbone", "61699b2e-d327-4a01-9f1e-0ea8c3f06bc6"),
        ];

        values
            .into_iter()
            .map(|(username, uuid)| {
                (
                    *PlayerUuid::new_with_online_username(username)
                        .unwrap()
                        .as_uuid(),
                    Uuid::try_parse(uuid).unwrap(),
                )
            })
            .for_each(|(uuid1, uuid2)| assert_eq!(uuid1, uuid2));
    }

    #[cfg(feature = "online")]
    #[test]
    fn online_uuids_to_names() {
        let values = vec![
            ("Notch", "069a79f4-44e9-4726-a5be-fca90e38aaf5"),
            ("Dinnerbone", "61699b2e-d327-4a01-9f1e-0ea8c3f06bc6"),
        ];

        values
            .into_iter()
            .map(|(username, uuid)| {
                (
                    username,
                    PlayerUuid::new_with_uuid(Uuid::try_parse(uuid).unwrap())
                        .unwrap()
                        .unwrap_online()
                        .get_username()
                        .unwrap(),
                )
            })
            .for_each(|(name1, name2)| assert_eq!(name1, name2));
    }
}
