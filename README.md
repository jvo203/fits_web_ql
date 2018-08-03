# fits_web_ql
A trial Rust re-implementation of the C/C++ FITSWebQL

# IMPORTANT
After cloning the fits_web_ql repository the 809MB-large spectral lines database file needs to be downloaded from

http://jvo.nao.ac.jp/~chris/splatalogue_v3.db

and placed inside the fits_web_ql directory.

# How to run a local version (Personal Edition)
cd into the fits_web_ql directory and execute

cargo run --release

# How to run on the server (only at the Japanese Virtual Observatory)
cd into the fits_web_ql directory and execute

cargo run --features=server --release
