NATIVE = native
THOR = $(HOME)/thor
#--target=avx2-i32x16
simd:
	ispc -O3 --pic --opt=fast-math --addressing=64 src/fits.ispc -o $(NATIVE)/fits.o -h $(NATIVE)/fits.h
	#rm $(NATIVE)/libfits.a
	ar -cqs $(NATIVE)/libfits.a $(NATIVE)/fits.o

thor:
	ar -cqs $(NATIVE)/libthorenc.a $(THOR)/common/*.o $(THOR)/enc/*.o

#-s TOTAL_MEMORY=$(EMCC_TOTAL_MEMORY)
#-s ALLOW_MEMORY_GROWTH=1
EMCC_TOTAL_MEMORY=536870912

#WASM_STRING = WASM2019-02-08.1
WASM_STRING = WASM2020-06-22.0

hevc:
#em++ -O3 -Wno-deprecated -s ASSERTIONS=1 -s ALLOW_MEMORY_GROWTH=1 -s EXTRA_EXPORTED_RUNTIME_METHODS='["cwrap"]' -s EXPORTED_FUNCTIONS="['_malloc','_free']"  -I$(HOME)/jctvc-hm/source/Lib $(HOME)/jctvc-hm/source/Lib/TLibCommon/*.cpp $(HOME)/jctvc-hm/source/Lib/TLibDecoder/*.cpp src/colourmap.c src/hevc_decoder.cpp -o build/hevc.js
	emcc -O3 -Wno-implicit-function-declaration -DARCH_X86=0 -DHAVE_FAST_UNALIGNED=0 -DFF_MEMORY_POISON=0x2a -s ERROR_ON_UNDEFINED_SYMBOLS=0 -s ALLOW_MEMORY_GROWTH=1 -s EXTRA_EXPORTED_RUNTIME_METHODS='["cwrap"]' -s "EXPORTED_FUNCTIONS=['_malloc','_free','_hevc_init','_hevc_destroy','_hevc_decode_nal_unit']" -I./FFmpeg -I./FFmpeg/libavutil -Isrc FFmpeg/libavutil/mastering_display_metadata.c FFmpeg/libavutil/dict.c FFmpeg/libavutil/display.c FFmpeg/libavutil/frame.c FFmpeg/libavutil/channel_layout.c FFmpeg/libavutil/samplefmt.c FFmpeg/libavutil/avstring.c FFmpeg/libavutil/md5.c FFmpeg/libavutil/rational.c FFmpeg/libavutil/mathematics.c FFmpeg/libavutil/opt.c FFmpeg/libavutil/eval.c FFmpeg/libavutil/time.c FFmpeg/libavutil/parseutils.c FFmpeg/libavutil/random_seed.c FFmpeg/libavutil/sha.c FFmpeg/libavutil/stereo3d.c FFmpeg/libavutil/hwcontext.c FFmpeg/libavutil/error.c FFmpeg/libavutil/file_open.c FFmpeg/libavutil/reverse.c FFmpeg/libavcodec/parser.c FFmpeg/libavcodec/parsers.c FFmpeg/libavcodec/bswapdsp.c FFmpeg/libavcodec/avpacket.c FFmpeg/libavcodec/options.c FFmpeg/libavcodec/allcodecs.c FFmpeg/libavcodec/codec_desc.c FFmpeg/libavcodec/decode.c FFmpeg/libavcodec/bsf.c FFmpeg/libavcodec/bitstream_filters.c FFmpeg/libavcodec/hevc_refs.c FFmpeg/libavcodec/hevcdec.c FFmpeg/libavcodec/hevc_cabac.c FFmpeg/libavcodec/hevc_filter.c FFmpeg/libavcodec/hevcdsp.c FFmpeg/libavcodec/hevc_mvs.c FFmpeg/libavcodec/hevcpred.c FFmpeg/libavcodec/cabac.c FFmpeg/libavcodec/videodsp.c FFmpeg/libavcodec/profiles.c FFmpeg/libavcodec/null_bsf.c FFmpeg/libavcodec/hevc_parse.c FFmpeg/libavcodec/hevc_parser.c FFmpeg/libavcodec/hevc_ps.c FFmpeg/libavutil/buffer.c FFmpeg/libavutil/pixdesc.c FFmpeg/libavutil/mem.c FFmpeg/libavutil/imgutils.c FFmpeg/libavutil/log.c FFmpeg/libavutil/bprint.c FFmpeg/libavutil/intmath.c FFmpeg/libavutil/log2_tab.c FFmpeg/libavcodec/h2645_parse.c FFmpeg/libavcodec/utils.c FFmpeg/libavcodec/hevc_sei.c FFmpeg/libavcodec/golomb.c FFmpeg/libavcodec/hevc_data.c src/colourmap.c src/hevc_decoder.c -o build/hevc_$(WASM_STRING).js --llvm-lto 1
#em++ -O3 -std=c++11 -D__STDC_CONSTANT_MACROS -s ALLOW_MEMORY_GROWTH=1 -s EXTRA_EXPORTED_RUNTIME_METHODS='["cwrap"]' -s EXPORTED_FUNCTIONS="['_malloc','_free']" -I$(HOME)/FFmpeg $(HOME)/FFmpeg/libavcodec/hevcdec.c src/colourmap.c src/hevc_decoder.c -o build/hevc.js
#emcc -O3 -s ALLOW_MEMORY_GROWTH=1 -s EXTRA_EXPORTED_RUNTIME_METHODS='["cwrap"]' -s EXPORTED_FUNCTIONS="['_malloc','_free']" -Ibuild/include -Lbuild/lib -lavcodec src/colourmap.c src/hevc_decoder.c -o build/hevc.js

vpx:
	emcc -O3 -s ALLOW_MEMORY_GROWTH=1 -s EXTRA_EXPORTED_RUNTIME_METHODS='["cwrap"]' -s EXPORTED_FUNCTIONS="['_malloc','_free']" -I/home/chris/ogv.js/build/js/root/include -L/home/chris/ogv.js/build/js/root/lib /home/chris/ogv.js/build/js/root/lib/libvpx.so src/colourmap.c src/vpx_decoder.c -o build/vpx.js

FPZIP_STRING = WASM2020-06-18.0

FPZIP=fpzip
SRC=$(FPZIP)/src/*.cpp
INCLUDE=-I./ -I./$(FPZIP)/include -I./$(FPZIP)/src
LIBRARY=
CXXFLAGS=-std=c++11 -O3
LDFLAGS=-lz --llvm-lto 1

EMFLAGS=--bind
EMFLAGS+=-s ALLOW_MEMORY_GROWTH=1
EMFLAGS+=-s NO_EXIT_RUNTIME=1
EMFLAGS+=-s NO_FILESYSTEM=1

FPZIP_FP = FPZIP_FP_FAST
FPZIP_BLOCK_SIZE = 0x1000
DEFS += -DFPZIP_BLOCK_SIZE=$(FPZIP_BLOCK_SIZE) -DFPZIP_FP=$(FPZIP_FP) $(FPZIP_CONV)
EMFLAGS+=-s MODULARIZE=1 -s 'EXPORT_NAME="FPZIP"'

fpunzip:
	em++ $(EMFLAGS) $(DEFS) $(SRC) src/main.cc -o build/fpzip.$(FPZIP_STRING).js $(INCLUDE) -L$(LIBRARY) $(CXXFLAGS) $(LDFLAGS)

#--post-js module-post.js
