#include <emscripten.h>

#include <libavcodec/hevc_parse.h>

#include <libavutil/common.h>

#include <string.h>

//colourmaps
#include "colourmap.h"

/*static AVCodec *codec;
static AVCodecContext *avctx;
static AVFrame *frame;*/

//extern AVCodec ff_hevc_decoder;

static HEVCParamSets params;
static HEVCSEI sei;

static int is_nalff;
static int nal_length_size;

EMSCRIPTEN_KEEPALIVE
static void hevc_init() {
    is_nalff = 1 ;
    nal_length_size = 0 ;

    /*codec = &ff_hevc_decoder;
    frame = NULL;

    avctx = avcodec_alloc_context3(codec);

	if (!avctx)
    {
    	printf("Failed to initialize HEVC decoder.\n");
        return ;
    }

    frame = av_frame_alloc();

    if (!frame)
    {
        printf("Failed to allocate HEVC frame.\n");
        return ;
    }

    avctx->err_recognition |= AV_EF_CRCCHECK;     
    if (avcodec_open2(avctx, codec, NULL) < 0)
    {
        av_frame_free(&frame);        
    }*/
}

EMSCRIPTEN_KEEPALIVE
static void hevc_destroy() {
    /*if (frame != NULL)
        av_frame_free(&frame);

    if (avctx != NULL)
        avcodec_free_context(&avctx);*/
}

EMSCRIPTEN_KEEPALIVE
static double hevc_decode_nal_unit(const unsigned char *data, size_t data_len) {
    double start = emscripten_get_now();
    double stop = 0.0 ;

    printf("HEVC: decoding a NAL unit of length %zu bytes\n", data_len);
    
    int err_recognition = 1;
    int apply_defdispwin = 0;

    int ret = ff_hevc_decode_extradata(data, data_len, &params, &sei, &is_nalff, &nal_length_size, err_recognition, apply_defdispwin, stdout);

    stop = emscripten_get_now();

    printf("[wasm hevc] ret = %d, is_nalff = %d, elapsed time %5.2f [ms]\n", ret, is_nalff, (stop-start)) ;

    double elapsed = stop - start;

	return elapsed ;
}