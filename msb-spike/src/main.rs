use microsandbox::{BaseSandbox, PythonSandbox, SandboxOptions, StartOptions};
use std::time::Instant;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

async fn sh(sandbox: &PythonSandbox, cmd: &str) -> Result<String> {
    let escaped = cmd.replace('\\', "\\\\").replace('"', "\\\"");
    let code = format!(
        "import subprocess; r = subprocess.run(['sh', '-c', \"{escaped}\"], capture_output=True, text=True, timeout=600); print(r.stdout + r.stderr)"
    );
    let exec = sandbox.run(&code).await?;
    exec.output().await
}

async fn py(sandbox: &PythonSandbox, code: &str) -> Result<String> {
    let exec = sandbox.run(code).await?;
    exec.output().await
}

const NIX: &str = "/nix/store/58p60gwspyw0032g8rn9w8pnh7i4r58r-nix-2.34.1/bin/nix";
const NIX_STORE_BIN: &str = "/nix/store/58p60gwspyw0032g8rn9w8pnh7i4r58r-nix-2.34.1/bin/nix-store";
const NIX_CERTS: &str = "/nix/store/2g4zfwsrkydpisqk3lz42cf9ak2lfvnc-nss-cacert-3.117/etc/ssl/certs/ca-bundle.crt";

async fn install_nix(sandbox: &PythonSandbox) -> Result<()> {
    sh(sandbox, &format!(
        "cd /tmp && \
         curl -sL https://releases.nixos.org/nix/nix-2.34.1/nix-2.34.1-aarch64-linux.tar.xz -o nix.tar.xz && \
         tar xf nix.tar.xz && \
         mkdir -p /nix/store && \
         cp -a /tmp/nix-2.34.1-aarch64-linux/store/* /nix/store/ && \
         mkdir -p /root/.cache/nix/tarball-cache-v2 /root/.cache/nix/fetcher-cache-v2 \
                  /nix/var/nix/db /nix/var/nix/gcroots /nix/var/nix/profiles \
                  /nix/var/nix/temproots /nix/var/nix/userpool /nix/var/nix/daemon-socket /etc/nix && \
         cd /root/.cache/nix/tarball-cache-v2 && git init --bare 2>&1 && cd / && \
         echo 'build-users-group =' > /etc/nix/nix.conf && \
         echo 'experimental-features = nix-command flakes' >> /etc/nix/nix.conf && \
         echo 'sandbox = false' >> /etc/nix/nix.conf && \
         echo 'ssl-cert-file = {NIX_CERTS}' >> /etc/nix/nix.conf && \
         echo '140.82.113.6 api.github.com' >> /etc/hosts && \
         echo '140.82.112.3 github.com' >> /etc/hosts && \
         echo '151.101.113.91 cache.nixos.org' >> /etc/hosts && \
         echo '151.101.113.91 channels.nixos.org' >> /etc/hosts && \
         echo '151.101.113.91 releases.nixos.org' >> /etc/hosts && \
         HOME=/root {NIX_STORE_BIN} --init 2>&1 && \
         HOME=/root {NIX_STORE_BIN} --load-db < /tmp/nix-2.34.1-aarch64-linux/.reginfo 2>&1 && \
         echo DONE"
    )).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let options = SandboxOptions::builder()
        .server_url("http://127.0.0.1:5555")
        .build();

    let mut sandbox = PythonSandbox::create_with_options(options).await?;
    sandbox.start(Some(StartOptions::default())).await?;
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    println!("=== installing nix ===");
    let t0 = Instant::now();
    install_nix(&sandbox).await?;
    println!("installed: {:?}", t0.elapsed());

    // Write nix expression via Python
    py(&sandbox, r#"
f = open('/tmp/build-image.nix', 'w')
f.write('let\n')
f.write('  pkgs = import (fetchTarball {\n')
f.write('    url = "https://github.com/NixOS/nixpkgs/archive/nixos-unstable.tar.gz";\n')
f.write('  }) {};\n')
f.write('in pkgs.dockerTools.buildLayeredImage {\n')
f.write('  name = "missouri-base";\n')
f.write('  tag = "latest";\n')
f.write('  contents = [ pkgs.bash pkgs.coreutils ];\n')
f.write('  config.Cmd = [ "${pkgs.bash}/bin/bash" ];\n')
f.write('}\n')
f.close()
print('written')
"#).await?;

    // First attempt — this will download nixpkgs and fail because the source
    // doesn't land properly. But the tarball will be cached.
    println!("\n=== first build attempt (downloads nixpkgs) ===");
    let t1 = Instant::now();
    let out = sh(&sandbox, &format!(
        "HOME=/root NIX_SSL_CERT_FILE={NIX_CERTS} {NIX} build -f /tmp/build-image.nix --impure --no-link --print-out-paths 2>&1 | tail -10"
    )).await?;
    println!("{out}");
    println!("attempt 1: {:?}", t1.elapsed());

    // Check the store path that nix complained about
    println!("\n=== checking source path ===");
    let out = sh(&sandbox, "ls /nix/store/ | grep source | head -5").await?;
    println!("source paths: {out}");
    let out = sh(&sandbox, "ls /nix/store/ap9dpkyzikzzh04259wlsvha2mw455x4-source/ 2>&1 | head -10").await?;
    println!("source contents: {out}");

    // Check disk space — maybe the VM ran out
    let out = sh(&sandbox, "df -h /nix/store/").await?;
    println!("disk: {out}");

    sandbox.stop().await?;
    println!("done");

    Ok(())
}
