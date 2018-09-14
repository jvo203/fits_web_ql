#include <emscripten.h>

#include <libavcodec/hevc_parse.h>

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
    //the "standard" way
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
    if (avcodec_open2(avctx, codec, NULL) < 0)
    {
        printf("Failed to open the HEVC coded.\n");
        av_frame_free(&frame);        
    }
}

EMSCRIPTEN_KEEPALIVE
static void hevc_destroy() {
    if (frame != NULL)
        av_frame_free(&frame);

    if (avctx != NULL)
        avcodec_free_context(&avctx);
}

EMSCRIPTEN_KEEPALIVE
static double hevc_decode_nal_unit(const unsigned char *data, size_t data_len) {
    double start = emscripten_get_now();
    double stop = 0.0 ;

    printf("HEVC: decoding a NAL unit of length %zu bytes\n", data_len);
    
    uint8_t* buf = realloc((void*)data, data_len + AV_INPUT_BUFFER_PADDING_SIZE);
    memset(buf + data_len, 0, AV_INPUT_BUFFER_PADDING_SIZE);

    AVPacket avpkt;    

    av_init_packet(&avpkt);
    avpkt.data = (uint8_t *)buf;
    avpkt.size = data_len;

    int ret = avcodec_send_packet(avctx, &avpkt);

    stop = emscripten_get_now();

    printf("[wasm hevc] ret = %d, elapsed time %5.2f [ms]\n", ret, (stop-start)) ;

    if( ret == AVERROR(EAGAIN) )
        printf("avcodec_receive_frame() is needed to remove decoded video frames\n");

    if( ret == AVERROR_EOF )
        printf("the decoder has been flushed\n");

    if( ret == AVERROR(EINVAL) )
        printf("codec not opened or requires flush\n");

    if( ret == AVERROR(ENOMEM) )
        printf("failed to add packet to internal queue etc.\n");

    bool has_frame = false ;

    //if( ret == AVERROR(EAGAIN) )
    {
        while( (ret = avcodec_receive_frame(avctx, frame)) == 0)
        {
            has_frame = true ;

            printf("decoded an HEVC frame\n");

            //apply a colourmap etc.

            av_frame_unref(frame);
        }

        printf("avcodec_receive_frame returned = %d\n", ret);
    }

    double elapsed = stop - start;

	return elapsed ;
}