use std::env;
use std::fs;
use std::path::Path;

const RESOURCES: &str = "resources";

fn agent_files(cwd: &str) -> String {
    // List files in resources/agents
    let files: Vec<_> = fs::read_dir(format!("{RESOURCES}/agents"))
        .unwrap()
        .flatten()
        .collect();
    // Include the content of those files in constants.rs as AGENTS with the include_bytes! macro
    let mut res_string = String::new();
    res_string.push_str("pub const AGENT_BYTES: [&[u8]; 4] = [");
    files.iter().for_each(|file| {
        res_string.push_str("include_bytes!(r#\"");
        res_string.push_str(cwd);
        res_string.push_str(file.path().to_str().unwrap());
        res_string.push_str("\"#),\n");
    });
    res_string.push_str("];\n");
    res_string
}

fn laser_files(cwd: &str) -> String {
    let mut res = String::new();
    for direction in ["horizontal", "vertical"] {
        let files: Vec<_> = fs::read_dir(format!("{RESOURCES}/lasers/{direction}"))
            .unwrap()
            .flatten()
            .collect();
        // Include the content of those files in constants.rs as AGENTS with the include_bytes! macro

        res.push_str(&format!(
            "pub const {}_LASER_BYTES: [&[u8]; 4] = [",
            direction.to_uppercase()
        ));
        files.iter().for_each(|file| {
            res.push_str("include_bytes!(r#\"");
            res.push_str(cwd);
            res.push_str(file.path().to_str().unwrap());
            res.push_str("\"#),\n");
        });
        res.push_str("];\n");
    }
    res
}

fn laser_source_files(cwd: &str) -> String {
    let mut res = String::new();
    for direction in ["east", "north", "south", "west"] {
        let files: Vec<_> = fs::read_dir(format!("{RESOURCES}/laser_sources/{direction}"))
            .unwrap()
            .flatten()
            .collect();
        // Include the content of those files in constants.rs as AGENTS with the include_bytes! macro
        let name = format!("LASER_SOURCE_{}_BYTES", direction.to_uppercase());
        let the_type = "[&[u8]; 4]";
        let mut value = String::from("[");
        files.iter().for_each(|file| {
            value.push_str(&format!(
                "include_bytes!(r#\"{cwd}{}\"#),\n",
                file.path().to_str().unwrap()
            ));
        });
        value.push(']');
        res.push_str(&make_const(&name, the_type, &value));
    }
    res
}

fn make_const(name: &str, the_type: &str, value: &str) -> String {
    format!("pub const {}: {} = {};\n", name, the_type, value)
}

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let cwd = env::current_dir().unwrap();
    // Append '/' to cwd
    let cwd = format!("{}/", cwd.to_str().unwrap());
    let dest_path = Path::new(&out_dir).join("constants.rs");

    let mut res = agent_files(&cwd);
    res.push_str(&laser_files(&cwd));
    res.push_str(&laser_source_files(&cwd));

    fs::write(dest_path, res).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
}
