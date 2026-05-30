extern crate embed_resource;

fn main() {
    println!("cargo:rerun-if-changed=backend.manifest");
    println!("cargo:rerun-if-changed=backend.rc");
    embed_resource::compile("backend.rc", embed_resource::NONE).manifest_optional().unwrap();
}