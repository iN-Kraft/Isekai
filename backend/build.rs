extern crate embed_resource;

fn main() {
    embed_resource::compile("backend.rc", embed_resource::NONE).manifest_optional().unwrap();
}