# fits_web_ql
A next-generation re-write of the C/C++ FITSWebQL in Rust. The previous C/C++ version can still be found here:

http://jvo.nao.ac.jp/~chris/fitswebql.html

# How to get started
clone the fits_web_ql project onto your computer with the git tool:

cd <your_projects_folder>

git clone https://github.com/jvo203/fits_web_ql.git

# IMPORTANT
After cloning the fits_web_ql repository the 809MB-large spectral lines database file needs to be downloaded from

http://jvo.nao.ac.jp/~chris/splatalogue_v3.db

and placed inside the fits_web_ql directory.

# How to Build, Prerequisites
First and foremost the Rust language must be installed:

https://www.rust-lang.org/install.html

##
install a free open-source Intel SPMD compiler (ispc)

cd <your_projects_folder>/fits_web_ql

run "make" (from within the fits_web_ql directory where the Makefile is located)

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
recently the following changes need to be applied manually in order to disable jpeg support

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

## WARNING
some Linux systems, for example Ubuntu, CentOS 6 and 7, need the following environment variable to be set before running fits_web_ql

export PKG_CONFIG_PATH=/usr/local/lib/pkgconfig

it is best to append this line into your .bashrc

# How to run a local version (Personal Edition)
cd into the fits_web_ql directory and execute

cargo run --release

# How to run the production server (only at the Japanese Virtual Observatory)
cd into the fits_web_ql directory and execute

cargo run --features 'server production' --release

or if you need to specify an alternative HTTP port

cargo run --features 'server production' --release -- --port 8000