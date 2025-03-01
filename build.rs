extern crate bindgen;
extern crate ispc;
extern crate metadeps;

use std::env;
use std::path::PathBuf;

fn main() {
    ispc::Config::new()
        .file("src/fits.ispc")
        .optimization_opt(ispc::opt::OptimizationOpt::FastMath)
        .addressing(ispc::opt::Addressing::A64)
        .woff()
        .compile("spmd");

    println!("cargo:rustc-link-search=native=/usr/local/lib");
    println!("cargo:rustc-link-lib=yuv");
    //println!("cargo:rustc-link-lib=x265");

    let libs = metadeps::probe().unwrap();
    let x265 = libs.get("x265").unwrap();

    //link_paths + libx265.so -> get the linkname libx265.so.160, then extract the last number
    let mut path = x265.link_paths[0].clone();
    path.push(PathBuf::from("libx265.so"));

    let apiver = if path.exists() {
        let link = path.read_link().unwrap();
        let name = link.to_str().unwrap();
        String::from(name.split(".").nth(2).unwrap())
    } else {
        //on macOS this should be libx265.dylib
        let mut path = x265.link_paths[0].clone();
        path.push(PathBuf::from("libx265.dylib"));

        if !path.exists() {
            panic!("cannot find a shared library for x265x");
        }

        let link = path.read_link().unwrap();
        let name = link.to_str().unwrap();
        String::from(name.split(".").nth(1).unwrap())
    };

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let builder = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .raw_line(format!(
            "pub unsafe fn x265_encoder_open(params: *mut x265_param) -> *mut x265_encoder {{
                               unsafe {{ x265_encoder_open_{}(params) }}
                          }}",
            apiver
        ))
        .header("wrapper.h")
        .clang_arg("-I")
        .clang_arg("/usr/local/include")
        .clang_arg("-I")
        .clang_arg(x265.include_paths[0].as_path().display().to_string())
        .clang_args(["-x", "c++", "-std=c++11"].iter());
    //.enable_cxx_namespaces()

    // Finish the builder and generate the bindings.
    let bindings = builder
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
