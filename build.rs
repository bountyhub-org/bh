use std::path::PathBuf;
use walkdir::WalkDir;

const PROTO_DIR: &str = "./proto";
const VENDOR_DIR: &str = "./vendor";

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let proto_dir = manifest_dir.join(PROTO_DIR);
    let vendor_dir = manifest_dir.join(VENDOR_DIR);
    let files = proto_files();

    println!("Files to compile: {:?}", files);

    connectrpc_build::Config::new()
        .files(&files)
        .includes(&[proto_dir.clone(), vendor_dir])
        .file_per_package(true)
        .include_file("connect.rs")
        .compile()
        .expect("compiling protos");
}

fn proto_files() -> Vec<PathBuf> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let proto_dir = manifest_dir.join(PROTO_DIR);

    let mut files = Vec::new();
    for entry in WalkDir::new(PROTO_DIR).into_iter().filter_map(Result::ok) {
        let path = entry.path();

        if path.is_file() {
            // Absolute paths let connectrpc-build strip the include prefix
            // and recover the proto-relative name for the descriptor set.
            let rel = path.strip_prefix(PROTO_DIR).unwrap();
            files.push(proto_dir.join(rel));
        }
    }

    files
}
