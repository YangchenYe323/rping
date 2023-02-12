use std::path::PathBuf;

fn main() {
  // todo(yangchen): Change the hard-coded path
  println!("cargo:rustc-link-lib=dylib=oping");
  println!("cargo:rustc-link-search=native=/usr/local/Cellar/liboping/1.10.0/lib");

  println!("cargo:rerun-if-changed=wrapper.h");

  let bindings = bindgen::Builder::default()
    .header("oping_wrapper.h")
    .parse_callbacks(Box::new(bindgen::CargoCallbacks))
    .generate()
    .expect("Unable to generate bindings");

  let out_path = PathBuf::from(std::env::var("OUT_DIR").unwrap());
  bindings
    .write_to_file(out_path.join("bindings.rs"))
    .expect("Couldn't write bindings!");
}
