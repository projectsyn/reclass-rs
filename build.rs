use std::env;
use std::fs;
use std::path::Path;

use rustc_version::version_meta;

fn main() {
    let rustc_version = version_meta().unwrap().short_version_string;
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("rustc_version.rs");
    fs::write(
        &dest_path,
        format!(r#"static RUSTC_VERSION: &str = "{rustc_version}";"#,),
    )
    .unwrap();
}
