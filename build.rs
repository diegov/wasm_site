use handlebars::Handlebars;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct UserInfoResponse {
    name: String,
}

fn main() {
    let mut handlebars = Handlebars::new();

    let source = read_file(&["html", "index.html.handlebars"]).unwrap();
    assert!(handlebars.register_template_string("t1", source).is_ok());

    let json = read_file(&["static", "sites.json"]).unwrap();

    let result: UserInfoResponse = serde_json::from_str(&json).unwrap();

    let mut data = BTreeMap::new();
    data.insert("title".to_string(), result.name);

    let final_html = handlebars.render("t1", &data).unwrap();

    let dest_path = get_path(&["static", "index.html"]);

    fs::write(&dest_path, final_html).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=html/*");
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
