cd /home/chris/FFmpeg
emmake make clean
emconfigure ./configure --prefix=/home/chris/projects/fits_web_ql/build --cc=emcc --nm=llvm-nm --ar=llvm-ar --disable-stripping --disable-inline-asm --disable-asm --disable-x86asm --disable-pthreads --disable-doc --disable-everything --enable-decoder=hevc
emmake make VERBOSE=1
emmake make install
