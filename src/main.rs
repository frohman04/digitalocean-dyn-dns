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

    client
        .get_domain(args.domain.clone())
        .expect("Error while contacting DigitalOcean")
        .expect("Unable to find domain in account");
    let record = client
        .get_record(args.domain, args.record, args.rtype)
        .expect("Error while contacting DigitalOcean")
        .expect("Unable to find record for domain with desired type");
    print!("{:?}", record)
}
