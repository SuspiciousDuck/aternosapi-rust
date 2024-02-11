# aternosapi(-rust)
It's my C++ project aternosapi, but rewritten in Rust. aternosapi is a library meant to control aternos-hosted servers programmatically using Rust. Remember, this **breaks Aternos' TOS for automation**, you can be banned.
# Current features
* Log into an existing aternos account
* List available servers for said account
* Start/stop servers
* Retrieve statuses of servers
* View players inside of server
* Query a specific server to access
# Usage
You can add this project as a dependency with
```bash
cargo add --git "https://github.com/SuspiciousDuck/aternosapi-rust"
```
Afterwards, you're free to use it as you please. <br>
Here's an example executable:
```rust
fn main() {
    let mut aternos = aternosapi::Instance::new();
    let result = aternos.login("username", "password");
    if result.is_err() {
        println!("Error signing in: {}", result.err().unwrap());
        std::process::exit(1);
    }
    aternos.fetch_servers();
    println!("{} Servers", aternos.servers.len());
    for server in aternos.servers {
        println!("Server: {}, id: {}", server.title, server.id);
    }
}
```