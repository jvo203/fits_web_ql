# fits_web_ql
A re-write of the C/C++ FITSWebQL in Rust. The previous C/C++ version can still be found here:

http://jvo.nao.ac.jp/~chris/fitswebql.html

The Rust version is now in a maintenance mode. A new C/C++ cluster edition - scaling to over 1TB-large files - is also in development: https://github.com/jvo203/FITSWebQL

![Alt text](fits_web_ql.jpeg?raw=true "FITSWebQLv4")

# How to Get Started
make sure the git tool is installed on your system:

https://git-scm.com/book/en/v2/Getting-Started-Installing-Git

download a stable release of fits_web_ql from

https://github.com/jvo203/fits_web_ql/releases

or alternatively clone the latest development version fits_web_ql onto your computer with the git tool:

cd <your_projects_folder>

git clone https://github.com/jvo203/fits_web_ql.git

# IMPORTANT
after cloning the fits_web_ql repository the 809MB-large spectral lines database needs to be downloaded from

http://jvo.nao.ac.jp/~chris/splatalogue_v3.db

and placed inside the fits_web_ql directory

(for example "wget http://jvo.nao.ac.jp/~chris/splatalogue_v3.db")

# How to Build, Prerequisites
First and foremost the Rust language version 1.31.0 or higher (Rust 2018 edition) must be installed:

macOS:

brew install rust

other platforms:

https://www.rust-lang.org/install.html

##
make and other command-line software development tools

Ubuntu Linux: open a terminal and type

sudo apt-get install build-essential

macOS: from the command-line

xcode-select --install

then install the Homebrew package manager:

https://coolestguidesontheplanet.com/installing-homebrew-on-macos-sierra-package-manager-for-unix-apps/

execute the following from the command line (you will be prompted for your password in order to complete the installation):

ruby -e "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/master/install)"

brew install cmake

##
install a free open-source Intel SPMD Program Compiler (ispc)

macOS:

brew install ispc

other platforms:

https://ispc.github.io

in the "Downloads" section select the binary corresponding to your platform

then place the extracted executable "ispc" in your PATH (for example ~/bin/ or /usr/local/bin)

##
install clang

macOS: this step can probably be skipped as clang should already be present, if not you should run "xcode-select --install" from the command-line to install the software development environment

Ubuntu Linux:

sudo apt install clang

##
install nasm and yasm assembler compilers

macOS:

brew install nasm yasm

Ubuntu Linux:

sudo apt-get install nasm yasm

##
install a libyuv library (YUV rescaling/image inversion):

git clone https://github.com/lemenkov/libyuv

cd libyuv

##########
the following changes need to be applied manually in order to disable jpeg support

https://xpra.org/trac/browser/xpra/trunk/osx/jhbuild/patches/libyuv-nojpeg.patch?rev=15432
##########

mkdir -p build

cd build

cmake -DCMAKE_POSITION_INDEPENDENT_CODE=ON ..

make

sudo make install

##
install Google's libvpx 1.7.0 or higher

macOS: "brew install libvpx"

other systems follow:

http://www.linuxfromscratch.org/blfs/view/svn/multimedia/libvpx.html

cd libvpx

./configure --enable-pic

make

sudo make install

(when compiling from source enforce -fPIC by means of the configure flag --enable-pic)

##
install x265 version 2.8 or higher

macOS: "brew install x265"

other systems follow:

http://www.linuxfromscratch.org/blfs/view/svn/multimedia/x265.html

cd x265_2.9

mkdir -p build

cd build

cmake ../source

make

sudo make install

please be sure to have nasm installed beforehand when building from source, plus NUMA API: numactl and numa development library libnuma (package libnuma-dev on Ubuntu)

##
install sqlite3

macOS: normally sqlite3 comes pre-installed in macOS, if not you may install it manually with "brew install sqlite3"

Ubuntu Linux: "sudo apt install libsqlite3-dev"

## WARNING
some Linux systems, for example Ubuntu, CentOS 6 and 7, need the following environment variables to be set before running fits_web_ql:

export PKG_CONFIG_PATH=/usr/local/lib/pkgconfig

export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:/usr/local/lib

it is best to append these lines into your .bashrc

# How to Run a Local Server (Personal Edition)
cd into the fits_web_ql directory and execute

cargo run --release

after a successful compilation (it may take some time!) point your web browser to http://localhost:8080

press CTRL+C to exit the program

to avoid needless compilation before running one can build it once with "cargo build --release" and then launch it many times by executing

target/release/fits_web_ql

or

target/release/fits_web_ql --port 8080 --interface 0.0.0.0 --home /a/path/to/your/FITS/mount

# How to Run the Production Server (only at the Japanese Virtual Observatory)
cd into the fits_web_ql directory and execute

cargo run --features 'jvo production cdn zfp' --release

or

cargo run --features 'jvo production cdn zfp' --release -- --path fitswebql_v4

# extra features and options

The "--features" option enables extra functionality. JVO-reserved features are "jvo" and "production". "cdn" can be used by anyone to speed up delivery of static resources by utilising a jsDelivr open-source content delivery network (https://www.jsdelivr.com). "cdn" is especially recommended if many users are accessing a remote FITSWebQL server. There is no need to use it on your personal computer. "zfp" enables ZFP compression for FITS data cubes held in an internal FITSWebQL cache in order to (theoretically) speed-up loading times (see a note at the end of this README). "ipp" enables use of the Intel Integrated Performance Primitives (IPP) library in some places (for example rescaling images/videos).

an alternative HTTP port

cargo run --features 'cdn' --release -- --port 8000

an alternative URL path (JVO-specific)

cargo run --features 'jvo production cdn zfp' --release -- --path fitswebql_v4

an alternative network interface (only needed to make the local version operate in a remote-server mode)

cargo run --release -- --interface 0.0.0.0

an alternative home directory (FITS data storage)

cargo run --release -- --home /a/path/to/your/FITS/mount

combined options

cargo run --features 'cdn' --release -- --port 8000 --interface 0.0.0.0 --home /a/path/to/your/FITS/mount

or

cargo build --features 'cdn' --release

target/release/fits_web_ql --port 8000 --interface 0.0.0.0 --home /a/path/to/your/FITS/mount

# How to Accelerate FITSWebQL

##
<i>FITSCACHE placement</i>

A hint how to speed-up FITSWebQL when using SSDs. It it best to clone fits_web_ql onto a directory residing on the fastest storage medium you have (ideally NVME SSD). Inside fits_web_ql there is an internal FITSCACHE directory where the program caches half-float-converted FITS files (applicable only to bitpix = -32).

The first time a FITS file is accessed it will be read from its original location (ideally from a fast NVME SSD). The second time somebody accesses that same FITS file, fits_web_ql will read the FITS header from the original location and then proceed to load half-float binary cache from the FITSCACHE directory.

So even if your large FITS files reside on a fast SSD, if fits_web_ql itself is located on a slow HDD second-time loads of FITS files will appear slow compared with first-time accesses.

One can also experiment with symbolic links to a separate FITSCACHE directory residing on a fast storage medium.

##
<i>enable the "zfp" feature</i>

i.e. cargo run --features 'zfp' --release

This feature replaces the half-float storage with ZFP compression (https://github.com/LLNL/zfp), which speeds-up second-time loads on multi-core systems. The HDD/SSD cache storage uses zfp 2d arrays instead of half-floats, which are converted to a half-float RAM storage upon loading. Decompressing data is very CPU intensive, hence this feature is only recommended if your server contains a sufficient number of CPU cores (i.e. >= than #memory channels). Otherwise speed savings from reading smaller file sizes will be eaten-up by increased CPU times of decompressing the data. cmake3 needs to be installed on your system.

A personal comment: ZFP compresses data using 4x4 blocks which introduces undesirable blocky artifacts at too high compression ratios. Compression/decompression speeds are not fast. In author's experience, compressing FITS data cubes with Radial Basis Functions (a terribly slow process by itself!) results in much better compression ratios with no visible artifacts. The author will continue experimenting with various compression methods (including wavelets).

##
<i>enable use of Intel IPP via an experimental feature "ipp"</i>

i.e. cargo run --features 'zfp ipp' --release

The Intel Integrated Performance Primitives (IPP) library can be obtained free of charge from https://software.intel.com/en-us/intel-ipp

IMPORTANT: please make sure that the <b>IPPROOT</b> environment variable is set following the IPP installation instructions (i.e. echo $IPPROOT
/opt/intel/compilers_and_libraries_2019.4.243/linux/ipp)
