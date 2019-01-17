# IMPORTANT: Rust 2018 Edition Upgrade

The codebase has been upgraded to Rust 2018. It requires Rust version 1.31.0 or higher. If you have an older version you can upgrade it by running "rustup update" in the command-line.

# fits_web_ql
A next-generation re-write of the C/C++ FITSWebQL in Rust. The previous C/C++ version can still be found here:

http://jvo.nao.ac.jp/~chris/fitswebql.html

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

##
install sqlite3

macOS: normally sqlite3 comes pre-installed in macOS, if not you may install it manually with "brew install sqlite3"

Ubuntu Linux: "sudo apt install libsqlite3-dev"

## WARNING
some Linux systems, for example Ubuntu, CentOS 6 and 7, need the following environment variables to be set before running fits_web_ql:

export PKG_CONFIG_PATH=/usr/local/lib/pkgconfig
export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:/usr/local/lib

it is best to append these lines into your .bashrc

# How to Run a Local Version (Personal Edition)
cd into the fits_web_ql directory and execute

cargo run --release

after a successful compilation (it may take some time!) point your web browser to http://localhost:8080

press CTRL+C to exit the program

# How to Run the Production Server (only at the Japanese Virtual Observatory)
cd into the fits_web_ql directory and execute

cargo run --features 'server production cdn' --release

# extra options
an alternative HTTP port

cargo run --features 'server production cdn' --release -- --port 8000

an alternative server URL path

cargo run --features 'server production cdn' --release -- --path fitswebql_v4

combined options

cargo run --features 'server production cdn' --release -- --port 8000 --path fitswebql_v4

an alternative network interface (only needed to make the local version run in a quasi-server mode)

cargo run --release -- --interface 0.0.0.0
