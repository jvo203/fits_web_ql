#include <emscripten.h>

#include <libavcodec/avcodec.h>
#include <libavutil/common.h>

#include <string.h>

//colourmaps
#include "colourmap.h"

static AVCodec *codec;
static AVCodecContext *avctx;
static AVFrame *frame;

extern AVCodec ff_hevc_decoder;

EMSCRIPTEN_KEEPALIVE
static void hevc_init() {
    codec = &ff_hevc_decoder;
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
    /* open it */
    if (avcodec_open2(avctx, codec, NULL) < 0)
    {
        av_frame_free(&frame);        
    }
}

static void hevc_destroy() {
    if (frame != NULL)
        av_frame_free(&frame);

    if (avctx != NULL)
        avcodec_free_context(&avctx);
}

