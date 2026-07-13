use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Default)]
struct ModNode {
    file: Option<String>,
    children: BTreeMap<String, ModNode>,
}

fn collect_protos(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_protos(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "proto") {
            out.push(path);
        }
    }
}

fn insert(root: &mut ModNode, package: &str) {
    let mut node = root;
    for segment in package.split('.') {
        node = node.children.entry(segment.to_string()).or_default();
    }
    node.file = Some(package.to_string());
}

fn render(node: &ModNode, buf: &mut String) {
    for (name, child) in &node.children {
        buf.push_str(&format!("pub mod {name} {{\n"));
        if let Some(file) = &child.file {
            buf.push_str(&format!(
                "include!(concat!(env!(\"OUT_DIR\"), \"/{file}.rs\"));\n"
            ));
        }
        render(child, buf);
        buf.push_str("}\n");
    }
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let protos_root = manifest_dir.join("protos");

    let mut proto_files = Vec::new();
    collect_protos(&protos_root, &mut proto_files);
    proto_files.sort();

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // prost never removes files from a previous run, so a package that gets renamed
    // or fixed (as opposed to added) would otherwise leave its old generated file
    // behind and silently pull it into the module tree below.
    for entry in fs::read_dir(&out_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            fs::remove_file(path).unwrap();
        }
    }

    prost_build::Config::new()
        .out_dir(&out_dir)
        .compile_protos(&proto_files, &[protos_root])
        .expect("failed to compile Android Auto / OpenAuto protobufs");

    // prost writes one `<package>.rs` file per unique `package` declaration, and
    // packages nest arbitrarily (e.g. `aap_protobuf.service.mediabrowser` is both a
    // file on its own and the parent of `aap_protobuf.service.mediabrowser.message`),
    // so the module tree is derived from the generated filenames rather than
    // hand-written to match the .proto package hierarchy.
    let mut packages = Vec::new();
    for entry in fs::read_dir(&out_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                packages.push(stem.to_string());
            }
        }
    }

    let mut root = ModNode::default();
    for package in &packages {
        insert(&mut root, package);
    }

    let mut buf = String::new();
    render(&root, &mut buf);
    fs::write(out_dir.join("mod_tree.rs"), buf).unwrap();

    println!("cargo:rerun-if-changed=protos");
}
