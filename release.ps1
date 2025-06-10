pwsh -Command {
    $ErrorActionPreference='Stop'
    Set-PSDebug -Trace 1

    cargo nextest run --no-tests=warn

    $env:CROSS_CONTAINER_UID=0
    $env:CROSS_CONTAINER_GID=0
    cross build --release --target aarch64-unknown-linux-gnu

    scp digitalocean-dyn-dns.service digitalocean-dyn-fw.service target/aarch64-unknown-linux-gnu/release/digitalocean-dyn-dns gilneas:~
    ssh gilneas -- "chmod +x ~/digitalocean-dyn-dns && sudo mv ~/digitalocean-dyn-dns /usr/local/bin/digitalocean-dyn-dns && sudo mv ~/digitalocean-dyn-dns.service  /etc/systemd/system/digitalocean-dyn-dns.service && sudo mv ~/digitalocean-dyn-fw.service  /etc/systemd/system/digitalocean-dyn-fw.service; sudo systemctl daemon-reload && sudo systemctl restart digitalocean-dyn-dns.service"
}
