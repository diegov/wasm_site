use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

#[derive(Deserialize, Serialize)]
pub struct Site {
    url: String,
    me: bool,
}

#[derive(Deserialize, Serialize)]
pub struct UserInfoResponse {
    name: String,
    homepage: String,
    sites: Vec<Site>,
}

fn main() -> io::Result<()> {
    const CUSTOM_SITES: &str = "assets/sites.json";
    const DEMO_SITES: &str = "sites.demo.json";

    fs::create_dir_all("static")?;

    let sites_to_use = if Path::new(CUSTOM_SITES).exists() {
        CUSTOM_SITES
    } else {
        // Also track custom sites file in case it's added later
        println!("cargo:rerun-if-changed={}", CUSTOM_SITES);
        DEMO_SITES
    };

    println!("cargo:rerun-if-changed={}", sites_to_use);

    fs::copy(sites_to_use, "static/sites.json")?;

    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);

    let source = read_file(&["html", "index.html.handlebars"]).unwrap();
    assert!(handlebars.register_template_string("t1", source).is_ok());

    let json = read_file(&["static", "sites.json"]).unwrap();

    let result: UserInfoResponse = serde_json::from_str(&json).unwrap();

    let final_html = handlebars.render("t1", &result).unwrap();

    let dest_path = get_path(&["static", "index.html"]);

    fs::write(&dest_path, final_html).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=html/*");

    Ok(())
}

fn read_file(path: &[&str]) -> io::Result<String> {
    let result = get_path(path);
    fs::read_to_string(result)
}

fn get_path(path: &[&str]) -> PathBuf {
    let root_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
    let mut result = PathBuf::from(root_dir);
    for part in path {
        result = result.join(part);
    }
    result
}
