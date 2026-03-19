use microsandbox::{BaseSandbox, PythonSandbox, SandboxOptions, StartOptions};
use std::time::Instant;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> Result<()> {
    let options = SandboxOptions::builder()
        .server_url("http://127.0.0.1:5555")
        .build();

    println!("--- creating + starting sandbox ---");
    let t0 = Instant::now();
    let mut sandbox = PythonSandbox::create_with_options(options).await?;
    sandbox.start(Some(StartOptions::default())).await?;
    println!("ready in {:?}", t0.elapsed());

    // Give the portal sidecar time to initialize
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // Test 1: basic python code execution
    println!("\n=== TEST 1: python code execution ===");
    let t1 = Instant::now();
    let exec = sandbox.run("print('hello from microsandbox')").await?;
    println!("[{:?}] output: {}", t1.elapsed(), exec.output().await?);

    // Test 2: run shell commands via python subprocess
    println!("\n=== TEST 2: shell commands via python subprocess ===");
    let exec = sandbox
        .run("import subprocess; r = subprocess.run(['uname', '-a'], capture_output=True, text=True); print(r.stdout.strip())")
        .await?;
    println!("uname: {}", exec.output().await?);

    // Test 3: network test via python
    println!("\n=== TEST 3: network connectivity ===");
    let exec = sandbox
        .run("import urllib.request; r = urllib.request.urlopen('http://example.com', timeout=5); print(f'status={r.status}')")
        .await?;
    println!("network: {}", exec.output().await?);
    println!("has_error: {}", exec.has_error());

    // Test 4: filesystem operations via python
    println!("\n=== TEST 4: filesystem write + read ===");
    let exec = sandbox
        .run("open('/tmp/test.txt', 'w').write('hello from inside the vm'); print(open('/tmp/test.txt').read())")
        .await?;
    println!("file: {}", exec.output().await?);

    // Test 5: try command.run (may fail with portal issue)
    println!("\n=== TEST 5: command.run (may fail) ===");
    let cmd = sandbox.command().await?;
    match cmd.run("uname -a", None, None).await {
        Ok(result) => {
            println!("stdout: {}", result.output().await?);
            println!("exit_code: {}", result.exit_code());
        }
        Err(e) => println!("command.run failed (expected): {e}"),
    }

    println!("\n--- stopping sandbox ---");
    sandbox.stop().await?;
    println!("done");

    Ok(())
}
