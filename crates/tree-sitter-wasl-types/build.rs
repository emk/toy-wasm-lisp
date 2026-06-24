use std::path::{Path, PathBuf};
use std::{env, fs};
use type_sitter_gen::{dylib_path, generate_nodes, generate_queries, super_nodes};

fn main() {
    // Common setup
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    println!("cargo::rerun-if-changed=build.rs");

    // Obligatory: in this and future lines, replace `../tree-sitter-wasl`
    // with the path to your grammar's folder, relative to the folder containing `Cargo.toml`
    println!("cargo::rerun-if-changed=../tree-sitter-wasl");

    // BUG: `type-sitter` fails to do a proper staleness check on this file, so
    // we need to clean it up ourselves.
    let _ = fs::remove_file(dylib_path(Path::new("../tree-sitter-wasl")));

    // To generate nodes
    let path = Path::new("../tree-sitter-wasl/src/node-types.json");
    fs::write(
        out_dir.join("nodes.rs"),
        generate_nodes(path).unwrap().into_string(),
    )
    .unwrap();

    // To generate queries
    fs::write(
        out_dir.join("queries.rs"),
        generate_queries(
            "../tree-sitter-wasl/queries",
            "../tree-sitter-wasl",
            // Replace with a different `syn::Path` if the nodes don't exist in a subling to `dest_path` named `nodes`
            &super_nodes(),
            // Replace with `true` if you are using the `yak-sitter` feature (by default, no)
            false,
        )
        .unwrap()
        .into_string(),
    )
    .unwrap();
}
