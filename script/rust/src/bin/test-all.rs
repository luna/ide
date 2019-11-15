
fn get_workspace_members(
    cargo_toml_root : toml::Value
) -> Vec<String> {

    match &cargo_toml_root["workspace"]["members"] {
        toml::Value::Array(list) => list.iter().map(|val| {
            match val {
                toml::Value::String(s) => s.clone(),
                _ => panic!("Workspace member is not a string"),
            }
        }).collect(),
        _ => panic!("Invalid workspace element")
    }
}

/// Call wasm-pack test for each workspace member
///
/// This function reads workspace members list from `Cargo.toml` in current
/// directory, and call `wasm-pack test` each member. All script arguments
/// are passed to wasm-pack test process.
fn main()
{
    let wasm_pack_args : Vec<String> = std::env::args().skip(1).collect();
    let cargo_toml_root = std::fs::read_to_string("Cargo.toml").unwrap()
        .parse::<toml::Value>().unwrap();

    for member in get_workspace_members(cargo_toml_root) {
        println!("Running tests for {}:", member);
        let status = std::process::Command::new("wasm-pack")
            .arg("test")
            .arg(&member)
            .args(&wasm_pack_args)
            .status()
            .unwrap();
        if !status.success() {
            panic!("Process for {} failed!{}", member, match status.code() {
                Some(code) => format!(" Code: {}", code),
                None => String::new()
            });
        }
    }
}
