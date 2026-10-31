//! Bundle every book under `library/` (`library/<shelf>/<id>.txt`) into
//! the program: `$OUT_DIR/books.rs` lists them for `library::all`.

use std::path::Path;

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("library");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut books = Vec::new();
    for shelf in ["own", "classics"] {
        let dir = root.join(shelf);
        println!("cargo:rerun-if-changed={}", dir.display());
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let path = e.path();
            if path.extension().is_some_and(|x| x == "txt") {
                let id = path.file_stem().unwrap().to_string_lossy().into_owned();
                books.push((shelf, id, path));
            }
        }
    }
    books.sort();
    let mut out =
        String::from("/// (shelf, id, text) of every bundled book.\npub const BOOKS: &[(&str, &str, &str)] = &[\n");
    for (shelf, id, path) in &books {
        out.push_str(&format!("    ({shelf:?}, {id:?}, include_str!({:?})),\n", path.display().to_string()));
    }
    out.push_str("];\n");
    let dest = Path::new(&std::env::var("OUT_DIR").unwrap()).join("books.rs");
    std::fs::write(dest, out).unwrap();
}
