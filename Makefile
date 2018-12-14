NATIVE = native
THOR = $(HOME)/thor
#--target=avx2-i32x16
simd:
	ispc -O3 --pic --opt=fast-math --addressing=32 src/fits.ispc -o $(NATIVE)/fits.o -h $(NATIVE)/fits.h
	#rm $(NATIVE)/libfits.a
	ar -cqs $(NATIVE)/libfits.a $(NATIVE)/fits.o

thor:
	ar -cqs $(NATIVE)/libthorenc.a $(THOR)/common/*.o $(THOR)/enc/*.o

#-s TOTAL_MEMORY=$(EMCC_TOTAL_MEMORY)
#-s ALLOW_MEMORY_GROWTH=1
EMCC_TOTAL_MEMORY=536870912

WASM_STRING = WASM2018-12-14.0

hevc:
#em++ -O3 -Wno-deprecated -s ASSERTIONS=1 -s ALLOW_MEMORY_GROWTH=1 -s EXTRA_EXPORTED_RUNTIME_METHODS='["cwrap"]' -s EXPORTED_FUNCTIONS="['_malloc','_free']"  -I$(HOME)/jctvc-hm/source/Lib $(HOME)/jctvc-hm/source/Lib/TLibCommon/*.cpp $(HOME)/jctvc-hm/source/Lib/TLibDecoder/*.cpp src/colourmap.c src/hevc_decoder.cpp -o build/hevc.js
	emcc -O3 -Wno-implicit-function-declaration -DARCH_X86=0 -DHAVE_FAST_UNALIGNED=0 -DFF_MEMORY_POISON=0x2a -s ALLOW_MEMORY_GROWTH=1 -s EXTRA_EXPORTED_RUNTIME_METHODS='["cwrap"]' -s EXPORTED_FUNCTIONS="['_malloc','_free']" -I$(HOME)/FFmpeg -I$(HOME)/FFmpeg/libavutil -Isrc $(HOME)/FFmpeg/libavutil/mastering_display_metadata.c $(HOME)/FFmpeg/libavutil/dict.c $(HOME)/FFmpeg/libavutil/display.c $(HOME)/FFmpeg/libavutil/frame.c $(HOME)/FFmpeg/libavutil/channel_layout.c $(HOME)/FFmpeg/libavutil/samplefmt.c $(HOME)/FFmpeg/libavutil/avstring.c $(HOME)/FFmpeg/libavutil/md5.c $(HOME)/FFmpeg/libavutil/rational.c $(HOME)/FFmpeg/libavutil/mathematics.c $(HOME)/FFmpeg/libavutil/opt.c $(HOME)/FFmpeg/libavutil/eval.c $(HOME)/FFmpeg/libavutil/time.c $(HOME)/FFmpeg/libavutil/parseutils.c $(HOME)/FFmpeg/libavutil/random_seed.c $(HOME)/FFmpeg/libavutil/sha.c $(HOME)/FFmpeg/libavutil/stereo3d.c $(HOME)/FFmpeg/libavutil/hwcontext.c $(HOME)/FFmpeg/libavutil/error.c $(HOME)/FFmpeg/libavutil/file_open.c $(HOME)/FFmpeg/libavutil/reverse.c $(HOME)/FFmpeg/libavcodec/parser.c $(HOME)/FFmpeg/libavcodec/parsers.c $(HOME)/FFmpeg/libavcodec/bswapdsp.c $(HOME)/FFmpeg/libavcodec/avpacket.c $(HOME)/FFmpeg/libavcodec/options.c $(HOME)/FFmpeg/libavcodec/allcodecs.c $(HOME)/FFmpeg/libavcodec/codec_desc.c $(HOME)/FFmpeg/libavcodec/decode.c $(HOME)/FFmpeg/libavcodec/bsf.c $(HOME)/FFmpeg/libavcodec/bitstream_filters.c $(HOME)/FFmpeg/libavcodec/hevc_refs.c $(HOME)/FFmpeg/libavcodec/hevcdec.c $(HOME)/FFmpeg/libavcodec/hevc_cabac.c $(HOME)/FFmpeg/libavcodec/hevc_filter.c $(HOME)/FFmpeg/libavcodec/hevcdsp.c $(HOME)/FFmpeg/libavcodec/hevc_mvs.c $(HOME)/FFmpeg/libavcodec/hevcpred.c $(HOME)/FFmpeg/libavcodec/cabac.c $(HOME)/FFmpeg/libavcodec/videodsp.c $(HOME)/FFmpeg/libavcodec/profiles.c $(HOME)/FFmpeg/libavcodec/null_bsf.c $(HOME)/FFmpeg/libavcodec/hevc_parse.c $(HOME)/FFmpeg/libavcodec/hevc_parser.c $(HOME)/FFmpeg/libavcodec/hevc_ps.c $(HOME)/FFmpeg/libavcodec/hevcdec.c $(HOME)/FFmpeg/libavutil/buffer.c $(HOME)/FFmpeg/libavutil/pixdesc.c $(HOME)/FFmpeg/libavutil/mem.c $(HOME)/FFmpeg/libavutil/imgutils.c $(HOME)/FFmpeg/libavutil/log.c $(HOME)/FFmpeg/libavutil/bprint.c $(HOME)/FFmpeg/libavutil/intmath.c $(HOME)/FFmpeg/libavutil/log2_tab.c $(HOME)/FFmpeg/libavcodec/h2645_parse.c src/colourmap.c $(HOME)/FFmpeg/libavcodec/h2645_parse.c $(HOME)/FFmpeg/libavcodec/utils.c $(HOME)/FFmpeg/libavcodec/hevc_sei.c $(HOME)/FFmpeg/libavcodec/golomb.c $(HOME)/FFmpeg/libavcodec/hevc_data.c src/colourmap.c src/hevc_decoder.c -o build/hevc_$(WASM_STRING).js
#em++ -O3 -std=c++11 -D__STDC_CONSTANT_MACROS -s ALLOW_MEMORY_GROWTH=1 -s EXTRA_EXPORTED_RUNTIME_METHODS='["cwrap"]' -s EXPORTED_FUNCTIONS="['_malloc','_free']" -I$(HOME)/FFmpeg $(HOME)/FFmpeg/libavcodec/hevcdec.c src/colourmap.c src/hevc_decoder.c -o build/hevc.js
#emcc -O3 -s ALLOW_MEMORY_GROWTH=1 -s EXTRA_EXPORTED_RUNTIME_METHODS='["cwrap"]' -s EXPORTED_FUNCTIONS="['_malloc','_free']" -Ibuild/include -Lbuild/lib -lavcodec src/colourmap.c src/hevc_decoder.c -o build/hevc.js

vpx:
	emcc -O3 -s ALLOW_MEMORY_GROWTH=1 -s EXTRA_EXPORTED_RUNTIME_METHODS='["cwrap"]' -s EXPORTED_FUNCTIONS="['_malloc','_free']" -I/home/chris/ogv.js/build/js/root/include -L/home/chris/ogv.js/build/js/root/lib /home/chris/ogv.js/build/js/root/lib/libvpx.so src/colourmap.c src/vpx_decoder.c -o build/vpx.js