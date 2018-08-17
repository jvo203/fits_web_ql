NATIVE = native
#--target=avx2-i32x16
simd:
	ispc -O3 --pic --opt=fast-math --addressing=32 src/fits.ispc -o $(NATIVE)/fits.o -h $(NATIVE)/fits.h
	#rm $(NATIVE)/libfits.a
	ar -cqs $(NATIVE)/libfits.a $(NATIVE)/fits.o

wasm:
	emcc -O3 -s ALLOW_MEMORY_GROWTH=1 -s EXTRA_EXPORTED_RUNTIME_METHODS='["cwrap"]' -s EXPORTED_FUNCTIONS="['_malloc','_free']" -I/home/chris/ogv.js/build/js/root/include -L/home/chris/ogv.js/build/js/root/lib /home/chris/ogv.js/build/js/root/lib/libvpx.so src/vpx_decoder.c -o build/vpx.js