#![forbid(unsafe_code)]

extern crate clap;
extern crate log;
extern crate reqwest;
extern crate serde;
extern crate serde_json;

mod cli;
mod digitalocean;
mod ip_retriever;

fn main() {
    let args = cli::Args::parse_args();
    let client = digitalocean::DigitalOceanClient::new(args.token);
    print!("{:?}", client.domain_exists(args.domain))
}
