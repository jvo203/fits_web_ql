#include <emscripten.h>

#include <libavcodec/hevc_ps.h>
#include <libavutil/common.h>

#include <string.h>

//colourmaps
#include "colourmap.h"

/*static AVCodec *codec;
static AVCodecContext *avctx;
static AVFrame *frame;*/

//extern AVCodec ff_hevc_decoder;

static HEVCParamSets params;

EMSCRIPTEN_KEEPALIVE
static void hevc_init() {
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

    stop = emscripten_get_now();

    printf("[wasm hevc] elapsed time %5.2f [ms]\n", (stop-start)) ;

    double elapsed = stop - start;

	return elapsed ;
}