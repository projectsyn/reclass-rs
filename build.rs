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
        format!(
            r#"
#[cfg_attr(target_os="linux", link_section=".rustc_version")]
#[cfg_attr(target_os="macos", link_section="__NOTE,__rustc_version")]
#[used]
#[no_mangle]
pub static RUSTC_VERSION: [u8; {}] = *b"{}";
    "#,
            rustc_version.len(),
            rustc_version
        ),
    )
    .unwrap();
}
