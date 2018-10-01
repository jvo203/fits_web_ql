# fits_web_ql
A trial Rust re-implementation of the C/C++ FITSWebQL

# IMPORTANT
After cloning the fits_web_ql repository the 809MB-large spectral lines database file needs to be downloaded from

http://jvo.nao.ac.jp/~chris/splatalogue_v3.db

and placed inside the fits_web_ql directory.

# How to Build, Prerequisites
install a free open-source Intel SPMD compiler (ispc) and then execute "make" from within the fits_webql_ql directory

##
install clang, i.e. for CentOS 7 please go to

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
install a libyuv library (YUV rescaling/image inversion):

git clone https://github.com/lemenkov/libyuv

cd libyuv

mkdir -p build

cd build

cmake -DCMAKE_POSITION_INDEPENDENT_CODE=ON ..

make

sudo make install

##
install Google's libvpx from for example https://www.webmproject.org/code/

git clone https://github.com/webmproject/libvpx

cd libvpx

./configure --enable-pic

make

sudo make install

(when compiling from source enforce -fPIC by means of the configure flag --enable-pic)

##
install x265 version 2.8

http://www.linuxfromscratch.org/blfs/view/8.3/multimedia/x265.html

## WARNING
many systems, for example Ubuntu, CentOS 6 and 7, need the following environment variable to be set before running fits_web_ql

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

# switching between VP9 and HEVC streaming video during development

cargo run --features 'server vp9' --release

cargo run --features 'server hevc' --release