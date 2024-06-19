#[derive(Debug)]
#[allow(dead_code)]
pub enum Error {
    Request(reqwest::Error),
    IpParse(std::net::AddrParseError),
    Update(String),
    Create(String),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Request(e)
    }
}

impl From<std::net::AddrParseError> for Error {
    fn from(e: std::net::AddrParseError) -> Self {
        Error::IpParse(e)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Request(_), Self::Request(_)) => false,
            (Self::IpParse(e1), Self::IpParse(e2)) => e1.to_string() == e2.to_string(),
            (Self::Update(e1), Self::Update(e2)) => e1 == e2,
            (Self::Create(e1), Self::Create(e2)) => e1 == e2,
            _ => false,
        }
    }
}
