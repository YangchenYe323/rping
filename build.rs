fn main() {
  println!("cargo:rustc-link-lib=dylib=oping");
  // alert(yangchen): Replace this with your local liboping library.
  println!("cargo:rustc-link-search=native=/usr/local/Cellar/liboping/1.10.0/lib");
}
