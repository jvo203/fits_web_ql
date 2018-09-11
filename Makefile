NATIVE = native
THOR = $(HOME)/thor
#--target=avx2-i32x16
simd:
	ispc -O3 --pic --opt=fast-math --addressing=32 src/fits.ispc -o $(NATIVE)/fits.o -h $(NATIVE)/fits.h
	#rm $(NATIVE)/libfits.a
	ar -cqs $(NATIVE)/libfits.a $(NATIVE)/fits.o

thor:
	ar -cqs $(NATIVE)/libthorenc.a $(THOR)/common/*.o $(THOR)/enc/*.o

hevc:
#em++ -O3 -Wno-deprecated -s ASSERTIONS=1 -s ALLOW_MEMORY_GROWTH=1 -s EXTRA_EXPORTED_RUNTIME_METHODS='["cwrap"]' -s EXPORTED_FUNCTIONS="['_malloc','_free']"  -I$(HOME)/jctvc-hm/source/Lib $(HOME)/jctvc-hm/source/Lib/TLibCommon/*.cpp $(HOME)/jctvc-hm/source/Lib/TLibDecoder/*.cpp src/colourmap.c src/hevc_decoder.cpp -o build/hevc.js
	emcc -O3 -Wno-implicit-function-declaration -DFF_MEMORY_POISON=0x2a -s ALLOW_MEMORY_GROWTH=1 -s EXTRA_EXPORTED_RUNTIME_METHODS='["cwrap"]' -s EXPORTED_FUNCTIONS="['_malloc','_free']" -I$(HOME)/FFmpeg -I$(HOME)/FFmpeg/libavutil -Isrc $(HOME)/FFmpeg/libavcodec/hevc_parse.c $(HOME)/FFmpeg/libavcodec/hevc_ps.c $(HOME)/FFmpeg/libavcodec/hevcdec.c $(HOME)/FFmpeg/libavutil/buffer.c $(HOME)/FFmpeg/libavutil/pixdesc.c $(HOME)/FFmpeg/libavutil/mem.c $(HOME)/FFmpeg/libavutil/imgutils.c $(HOME)/FFmpeg/libavutil/log.c $(HOME)/FFmpeg/libavutil/bprint.c $(HOME)/FFmpeg/libavutil/intmath.c $(HOME)/FFmpeg/libavutil/log2_tab.c $(HOME)/FFmpeg/libavcodec/h2645_parse.c src/colourmap.c $(HOME)/FFmpeg/libavcodec/h2645_parse.c $(HOME)/FFmpeg/libavcodec/utils.c $(HOME)/FFmpeg/libavcodec/hevc_sei.c $(HOME)/FFmpeg/libavcodec/golomb.c $(HOME)/FFmpeg/libavcodec/hevc_data.c src/colourmap.c src/hevc_decoder.c -o build/hevc.js
#em++ -O3 -std=c++11 -D__STDC_CONSTANT_MACROS -s ALLOW_MEMORY_GROWTH=1 -s EXTRA_EXPORTED_RUNTIME_METHODS='["cwrap"]' -s EXPORTED_FUNCTIONS="['_malloc','_free']" -I$(HOME)/FFmpeg $(HOME)/FFmpeg/libavcodec/hevcdec.c src/colourmap.c src/hevc_decoder.c -o build/hevc.js
#emcc -O3 -s ALLOW_MEMORY_GROWTH=1 -s EXTRA_EXPORTED_RUNTIME_METHODS='["cwrap"]' -s EXPORTED_FUNCTIONS="['_malloc','_free']" -Ibuild/include -Lbuild/lib -lavcodec src/colourmap.c src/hevc_decoder.c -o build/hevc.js

vpx:
	emcc -O3 -s ALLOW_MEMORY_GROWTH=1 -s EXTRA_EXPORTED_RUNTIME_METHODS='["cwrap"]' -s EXPORTED_FUNCTIONS="['_malloc','_free']" -I/home/chris/ogv.js/build/js/root/include -L/home/chris/ogv.js/build/js/root/lib /home/chris/ogv.js/build/js/root/lib/libvpx.so src/colourmap.c src/vpx_decoder.c -o build/vpx.js