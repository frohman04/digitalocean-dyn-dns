# digitalocean-dyn-dns

Utility for replicating your residential IP address into a DigitalOcean DNS entry.  Also capable of updating
DigitalOcean firewall rules to include your local IP address.

## Usage

By default, fetches your public IP address using ipinfo.io.  Can be modified to instead get the IP address of the local
machine when connecting to the internet.  (Not entirely sure what happens if you have multiple network interfaces that
have paths to the internet.)

### DNS

This tool is capable of setting the IP address (both A and AAAA records) for a DNS entry stored in DigitalOcean to the
IP address of the machine the command is run on.  This should be run periodically to account for your ISP changing your
IP address dynamically at any time of the day.

Example systemd unit file:

```toml
[Unit]
Description=DigitalOcean dynamic DNS updater
Wants=network-online.target
After=network-online.target

[Service]
Type=oneshot
Environment="DIGITAL_OCEAN_TOKEN=your_api_key"
ExecStart=/usr/local/bin/digitalocean-dyn-dns dns @ your.domain --rtype A
```

### Firewall

This tool is capable of updating the DigitalOcean firewall rules for a specified port.  In addition to adding your local
IP address to the rule, additional IPv4 and IPv6 addresses can be statically included in the rule, along with routes
directly to droplets, by their name.

Example systemd unit file:

```toml
[Unit]
Description=DigitalOcean dynamic DNS firewall updater
Wants=network-online.target
After=network-online.target

[Service]
Type=oneshot
Environment="DIGITAL_OCEAN_TOKEN=your_api_key"
ExecStart=/usr/local/bin/digitalocean-dyn-dns firewall wireguard 443 tcp --inbound --addresses 10.0.0.0/16,100.64.0.0/10,fd7a:115c:a1e0:ab12::/64,fe80::f864:e39a:5d9:0/64 --droplets my_machine
ExecStart=/usr/local/bin/digitalocean-dyn-dns firewall wireguard 80 tcp --inbound --addresses 10.0.0.0/16,100.64.0.0/10,fd7a:115c:a1e0:ab12::/64,fe80::f864:e39a:5d9:0/64 --droplets my_machine
```
