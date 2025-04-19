#!/bin/sh

set -e
set -x

cargo nextest run
if [ -z ${OS+x} ]; then
    # in Linux, use cross to build other platforms
    CROSS_CONTAINER_UID=0 CROSS_CONTAINER_GID=0 cross build --release --target aarch64-unknown-linux-gnu
else
    # in Windows, so need to uze zigbuild for aarch64 compat
    cargo zigbuild --release --target aarch64-unknown-linux-gnu
fi
scp digitalocean-dyn-dns.service digitalocean-dyn-fw.service target/aarch64-unknown-linux-gnu/release/digitalocean-dyn-dns gilneas:~
ssh gilneas -- "chmod +x ~/digitalocean-dyn-dns && sudo mv ~/digitalocean-dyn-dns /usr/local/bin/digitalocean-dyn-dns && sudo mv ~/digitalocean-dyn-dns.service  /etc/systemd/system/digitalocean-dyn-dns.service && sudo mv ~/digitalocean-dyn-fw.service  /etc/systemd/system/digitalocean-dyn-fw.service; sudo systemctl daemon-reload && sudo systemctl restart digitalocean-dyn-dns.service"
