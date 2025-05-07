use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::sync::{Arc, RwLock};
use std::{
    fmt::Display,
    net::{IpAddr, SocketAddr as StdSocketAddr},
};
/// Common url used through the project
#[derive(Clone, Hash, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Url(String);

impl Url {
    #[cfg(test)]
    pub fn get_ptr(str: &'static str) -> Arc<RwLock<Option<Url>>> {
        Arc::new(RwLock::new(Some(Url::from(str))))
    }

    pub fn get_port(&self) -> String {
        self.0.split(':').collect::<Vec<&str>>()[1].to_string()
    }
}

impl From<String> for Url {
    fn from(url: String) -> Self {
        Url(url)
    }
}

impl From<&String> for Url {
    fn from(url: &String) -> Self {
        Url(url.to_owned())
    }
}

impl Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl From<StdSocketAddr> for Url {
    fn from(u: StdSocketAddr) -> Self {
        Self(u.to_string())
    }
}

impl From<IpAddr> for Url {
    fn from(u: IpAddr) -> Self {
        Self(u.to_string())
    }
}

impl From<&'static str> for Url {
    fn from(u: &'static str) -> Self {
        Self(u.to_string())
    }
}
