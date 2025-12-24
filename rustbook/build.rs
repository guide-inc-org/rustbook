// build.rs - Tell cargo to recompile when template files change
fn main() {
    // Watch template files for changes
    println!("cargo:rerun-if-changed=templates/gitbook.css");
    println!("cargo:rerun-if-changed=templates/gitbook.js");
    println!("cargo:rerun-if-changed=templates/collapsible.js");
}
