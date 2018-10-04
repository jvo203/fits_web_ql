# fits_web_ql
A next-generation re-implementation of the C/C++ FITSWebQL in Rust. The previous C/C++ version can still be found here:

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
##
First and foremost the Rust language must be installed:

https://www.rust-lang.org/install.html

##
install a free open-source Intel SPMD compiler (ispc) and then execute "make" from within the fits_webql_ql directory

##
install clang (for macOS this step can probably be skipped as clang should already be present),

i.e. for CentOS 7 please go to

https://copr.fedorainfracloud.org/coprs/alonid/llvm-3.9.0/

as root add the following contents to /etc/yum.repos.d/epel.repo

[alonid-llvm-3.9.0]

name=Copr repo for llvm-3.9.0 owned by alonid

baseurl=https://copr-be.cloud.fedoraproject.org/results/alonid/llvm-3.9.0/epel-7-$basearch/

type=rpm-md

skip_if_unavailable=True

gpgcheck=1

gpgkey=https://copr-be.cloud.fedoraproject.org/results/alonid/llvm-3.9.0/pubkey.gpg

repo_gpgcheck=0

enabled=1

enabled_metadata=1

, then execute

sudo yum install clang-3.9.0

and add /opt/llvm-3.9.0/bin to your $PATH

and set LIBCLANG_PATH as well:

export PATH=/opt/llvm-3.9.0/bin:$PATH

export LIBCLANG_PATH=/opt/llvm-3.9.0/lib64

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
macOS: the following changes need to be applied manually in order to disable jpeg

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
install x265 version 2.8

macOS: "brew install x265"

other systems follow:

http://www.linuxfromscratch.org/blfs/view/svn/multimedia/x265.html

please be sure to have nasm installed beforehand when building from source, plus NUMA API: numactl and numa development library libnuma (package libnuma-dev on Ubuntu)

## WARNING
some Linux systems, for example Ubuntu, CentOS 6 and 7, need the following environment variable to be set before running fits_web_ql

export PKG_CONFIG_PATH=/usr/local/lib/pkgconfig

it is best to append this line into your .bashrc

# How to run a local version (Personal Edition)
cd into the fits_web_ql directory and execute

cargo run --release

# How to run on the production server (only at the Japanese Virtual Observatory)
cd into the fits_web_ql directory and execute

cargo run --features 'server production' --release

or if you need to specify an alternative HTTP port

cargo run --features 'server production' --release -- --port 8000