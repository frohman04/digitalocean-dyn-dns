use reqwest::blocking::ClientBuilder;

use std::io;
use std::net::{IpAddr, UdpSocket};

/// Get the IP address of the local network interface used to connect to the internet
pub fn get_local_ip() -> Result<IpAddr, io::Error> {
    // based on https://github.com/egmkang/local_ipaddress/blob/master/src/lib.rs

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("8.8.8.8:80")?;
    Ok(socket.local_addr()?.ip())
}

/// Get the IP address that is seen for this host on the internet
pub fn get_external_ip() -> Result<IpAddr, reqwest::Error> {
    let client = ClientBuilder::default()
        .build()
        .expect("Unable to construct HTTP client");
    Ok(client
        .get("http://ipinfo.io/ip")
        .send()?
        .text()?
        .trim()
        .parse::<IpAddr>()
        .unwrap())
}
