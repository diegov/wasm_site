use chrono::Utc;
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
    last_modified: Option<String>,
}

const STATIC_DIR: &str = "static";

fn main() -> io::Result<()> {
    const CUSTOM_SITES: &str = "assets/sites.json";
    const DEMO_SITES: &str = "sites.demo.json";

    fs::create_dir_all(STATIC_DIR)?;

    let sites_to_use = if Path::new(CUSTOM_SITES).exists() {
        CUSTOM_SITES
    } else {
        // Also track custom sites file in case it's added later
        println!("cargo:rerun-if-changed={}", CUSTOM_SITES);
        DEMO_SITES
    };

    println!("cargo:rerun-if-changed={}", sites_to_use);

    fs::copy(sites_to_use, Path::new(STATIC_DIR).join("sites.json"))?;

    let mut handlebars = Handlebars::new();
    handlebars.set_strict_mode(true);

    let json = read_file(&[STATIC_DIR, "sites.json"]).unwrap();
    let mut user_info: UserInfoResponse = serde_json::from_str(&json).unwrap();
    user_info.last_modified = Some(Utc::now().to_rfc3339());

    render_template_to_file(
        &mut handlebars,
        "html",
        "index.html.handlebars",
        "index.html",
        &user_info,
    );

    render_template_to_file(
        &mut handlebars,
        "html",
        "sitemap.xml.handlebars",
        "sitemap.xml",
        &user_info,
    );

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=html/*");

    Ok(())
}

fn render_template_to_file(
    handlebars: &mut Handlebars,

    directory: &str,
    template_filename: &str,
    output_filename: &str,
    user_info: &UserInfoResponse,
) {
    let source = read_file(&[directory, template_filename]).unwrap();
    assert!(handlebars
        .register_template_string(template_filename, source)
        .is_ok());
    let final_text = handlebars.render(template_filename, &user_info).unwrap();
    let dest_path = get_path(&[STATIC_DIR, output_filename]);
    fs::write(&dest_path, final_text).unwrap();
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
