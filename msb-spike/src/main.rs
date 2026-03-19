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

const NIX: &str = "/nix/store/58p60gwspyw0032g8rn9w8pnh7i4r58r-nix-2.34.1/bin/nix";
const NIX_STORE: &str = "/nix/store/58p60gwspyw0032g8rn9w8pnh7i4r58r-nix-2.34.1/bin/nix-store";
const NIX_CERTS: &str = "/nix/store/2g4zfwsrkydpisqk3lz42cf9ak2lfvnc-nss-cacert-3.117/etc/ssl/certs/ca-bundle.crt";

#[tokio::main]
async fn main() -> Result<()> {
    let options = SandboxOptions::builder()
        .server_url("http://127.0.0.1:5555")
        .build();

    let mut sandbox = PythonSandbox::create_with_options(options).await?;
    sandbox.start(Some(StartOptions::default())).await?;
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // Install nix — all in one big command to avoid state issues between calls
    println!("=== installing nix (single command) ===");
    let t0 = Instant::now();
    let out = sh(&sandbox, &format!(
        "cd /tmp && \
         curl -sL https://releases.nixos.org/nix/nix-2.34.1/nix-2.34.1-aarch64-linux.tar.xz -o nix.tar.xz && \
         tar xf nix.tar.xz && \
         mkdir -p /nix/store && \
         cp -a /tmp/nix-2.34.1-aarch64-linux/store/* /nix/store/ && \
         mkdir -p /root/.cache/nix/tarball-cache-v2 /root/.cache/nix/fetcher-cache-v2 \
                  /nix/var/nix/db /nix/var/nix/gcroots /nix/var/nix/profiles \
                  /nix/var/nix/temproots /nix/var/nix/userpool /nix/var/nix/daemon-socket /etc/nix && \
         cd /root/.cache/nix/tarball-cache-v2 && git init --bare 2>&1 && \
         echo 'build-users-group =' > /etc/nix/nix.conf && \
         echo 'experimental-features = nix-command flakes' >> /etc/nix/nix.conf && \
         echo 'sandbox = false' >> /etc/nix/nix.conf && \
         echo 'ssl-cert-file = {NIX_CERTS}' >> /etc/nix/nix.conf && \
         HOME=/root {NIX_STORE} --init 2>&1 && \
         HOME=/root {NIX_STORE} --load-db < /tmp/nix-2.34.1-aarch64-linux/.reginfo 2>&1 && \
         HOME=/root NIX_SSL_CERT_FILE={NIX_CERTS} {NIX} --version 2>&1"
    )).await?;
    println!("{out}");
    println!("install: {:?}", t0.elapsed());

    // Now try nix build
    println!("\n=== nix build nixpkgs#hello ===");
    let t1 = Instant::now();
    let out = sh(&sandbox, &format!(
        "HOME=/root NIX_SSL_CERT_FILE={NIX_CERTS} {NIX} build nixpkgs#hello --no-link --print-out-paths 2>&1 | tail -20"
    )).await?;
    println!("{out}");
    println!("build: {:?}", t1.elapsed());

    sandbox.stop().await?;
    println!("done");

    Ok(())
}
