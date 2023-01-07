//! This library provides functionality for converting usernames to and from Minecraft UUIDs,
//! including support for offline and online players.  
//! You may choose to disable either the `offline` or `online` features if you don't need them.
//!
//! To start, head over to [`PlayerUuid`] or look at some of the examples in this crate.

#[cfg(not(any(feature = "online", feature = "offline")))]
compile_error!("please select at least one feature (uuid-mc)");

use thiserror::Error;
use uuid::Version;
pub use uuid::{self, Uuid};

/// This library's own error enum, which is returned by every function that returns a [`Result`](std::result::Result).
#[derive(Debug, Error)]
pub enum Error {
    /// An error that signifies that the user has provided an invalid UUID, be it in the wrong format or a non-existent UUID if in an online context.
    #[error("invalid uuid")]
    InvalidUuid,

    /// An error that signifies that the user has provided an invalid username to Mojang's API.
    #[error("invalid username")]
    InvalidUsername,

    /// A Transport error from [`ureq`].
    #[cfg(feature = "online")]
    #[error("ureq transport error: {0}")]
    Transport(ureq::Transport),

    /// An unknown error used as a catch-all.
    #[error("unknown")]
    Unknown,
}

type Result<T> = std::result::Result<T, Error>;

/// A struct that represents a UUID with an online format (UUID v4).
#[cfg(feature = "online")]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct OnlineUuid(Uuid);

/// A struct that represents a UUID with an offline format (UUID v3).
#[cfg(feature = "offline")]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct OfflineUuid(Uuid);

/// An enum that can represent both kinds of UUIDs.
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
    /// Uses the Mojang API to fetch the username belonging to this UUID.
    ///
    /// # Errors
    /// If there is no user that corresponds to the provided UUID, an [`Error::InvalidUuid`] is returned.  
    /// Otherwise, an [`Error::Transport`] can be returned in case of network failure.
    ///
    /// # Examples
    /// To fetch the user belonging to an arbitrary UUID, you can do:
    /// ```rust
    /// use uuid::Uuid;
    /// use uuid_mc::{PlayerUuid, OnlineUuid};
    ///
    /// # fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    /// let uuid = Uuid::try_parse("069a79f4-44e9-4726-a5be-fca90e38aaf5")?;
    /// let player_uuid = PlayerUuid::new_with_uuid(uuid)?;
    ///
    /// let name = player_uuid.unwrap_online().get_username()?;
    /// assert_eq!(name, "Notch");
    /// # Ok(())
    /// # }
    /// ```
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

    /// Returns the inner [Uuid].
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Returns the inner UUID, as a byte array. This is just a convenience function around `self.as_uuid().as_bytes()`.
    pub fn as_bytes(&self) -> &[u8; 16] {
        self.as_uuid().as_bytes()
    }
}

#[cfg(feature = "offline")]
impl OfflineUuid {
    /// Returns the inner [Uuid].
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Returns the inner UUID, as a byte array. This is just a convenience function around `self.as_uuid().as_bytes()`.
    pub fn as_bytes(&self) -> &[u8; 16] {
        self.as_uuid().as_bytes()
    }
}

impl PlayerUuid {
    /// Creates a new instance using the username of an online player, by polling the Mojang API.
    ///
    /// # Errors
    /// If there is no user that corresponds to the provided username, an [`Error::InvalidUsername`] is returned.  
    /// Otherwise, an [`Error::Transport`] can be returned in case of network failure.
    ///
    /// # Examples
    /// To fetch the UUID of an online user:
    /// ```rust
    /// use uuid::Uuid;
    /// use uuid_mc::PlayerUuid;
    ///
    /// # fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    /// let uuid = PlayerUuid::new_with_online_username("Notch")?;
    /// let uuid = uuid.as_uuid();
    /// let expected = Uuid::try_parse("069a79f4-44e9-4726-a5be-fca90e38aaf5")?;
    /// assert_eq!(uuid, &expected);
    /// # Ok(())
    /// # }
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

    /// Creates a new instance using the username of an offline player.
    ///
    /// # Examples
    /// To fetch the UUID of an offline user:
    /// ```rust
    /// use uuid::Uuid;
    /// use uuid_mc::PlayerUuid;
    ///
    /// # fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    /// let uuid = PlayerUuid::new_with_offline_username("boolean_coercion");
    /// let uuid = uuid.as_uuid();
    /// let expected = Uuid::try_parse("db62bdfb-eddc-3acc-a14e-c703aba52549")?;
    /// assert_eq!(uuid, &expected);
    /// # Ok(())
    /// # }
    #[cfg(feature = "offline")]
    pub fn new_with_offline_username(username: &str) -> Self {
        let mut hash = md5::compute(format!("OfflinePlayer:{}", username)).0;
        hash[6] = hash[6] & 0x0f | 0x30; // uuid version 3
        hash[8] = hash[8] & 0x3f | 0x80; // RFC4122 variant

        let uuid = Uuid::from_bytes(hash);
        Self::Offline(OfflineUuid(uuid))
    }

    /// Creates a new instance using an already existing [`Uuid`].
    ///
    /// # Errors
    /// In case the provided Uuid is neither offline (v3) or online (v4), an [`Error::InvalidUuid`] is returned.
    ///
    /// # Examples
    /// To test whether a given Uuid is of the offline or online format:
    /// ```rust
    /// use uuid::Uuid;
    /// use uuid_mc::PlayerUuid;
    ///
    /// # #[cfg(all(feature = "online", feature = "offline"))]
    /// # fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    /// let uuid_offline = Uuid::try_parse("db62bdfb-eddc-3acc-a14e-c703aba52549")?;
    /// let uuid_online = Uuid::try_parse("61699b2e-d327-4a01-9f1e-0ea8c3f06bc6")?;
    /// let player_uuid_offline = PlayerUuid::new_with_uuid(uuid_offline)?;
    /// let player_uuid_online = PlayerUuid::new_with_uuid(uuid_online)?;
    ///
    /// assert!(matches!(player_uuid_offline, PlayerUuid::Offline(_)));
    /// assert!(matches!(player_uuid_online, PlayerUuid::Online(_)));
    /// # Ok(())
    /// # }
    /// # #[cfg(not(all(feature = "online", feature = "offline")))] fn main() {}
    pub fn new_with_uuid(uuid: Uuid) -> Result<Self> {
        match uuid.get_version() {
            #[cfg(feature = "online")]
            Some(Version::Random) => Ok(Self::Online(OnlineUuid(uuid))),
            #[cfg(feature = "offline")]
            Some(Version::Md5) => Ok(Self::Offline(OfflineUuid(uuid))),
            _ => Err(Error::InvalidUuid),
        }
    }

    /// Returns the inner [`Uuid`].
    pub fn as_uuid(&self) -> &Uuid {
        match self {
            #[cfg(feature = "online")]
            Self::Online(uuid) => uuid.as_uuid(),
            #[cfg(feature = "offline")]
            Self::Offline(uuid) => uuid.as_uuid(),
        }
    }

    /// Returns the inner UUID, as a byte array. This is just a convenience function around `self.as_uuid().as_bytes()`.
    pub fn as_bytes(&self) -> &[u8; 16] {
        self.as_uuid().as_bytes()
    }

    /// Similar to [`Result::unwrap`](std::result::Result::unwrap), this function returns the inner [`OfflineUuid`].
    ///
    /// # Panics
    /// If the inner UUID is not an offline one.
    #[cfg(feature = "offline")]
    pub fn unwrap_offline(self) -> OfflineUuid {
        match self {
            #[cfg(feature = "online")]
            Self::Online(_) => panic!("unwrap_offline called on an online uuid"),
            Self::Offline(uuid) => uuid,
        }
    }

    /// Similar to [`Result::unwrap`](std::result::Result::unwrap), this function returns the inner [`OnlineUuid`].
    ///
    /// # Panics
    /// If the inner UUID is not an online one.
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
