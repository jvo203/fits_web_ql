#include <emscripten.h>

#include <vpx/vpx_decoder.h>
#include <vpx/vp8dx.h>

#include <stdio.h> 

static vpx_codec_ctx_t codec;

EMSCRIPTEN_KEEPALIVE
static int vpx_version() {
	return VPX_DECODER_ABI_VERSION ;
}

EMSCRIPTEN_KEEPALIVE
static void vpx_init() {
	if (vpx_codec_dec_init(&codec, vpx_codec_vp9_dx(), NULL, 0))
    	printf("Failed to initialize decoder.\n");
}

EMSCRIPTEN_KEEPALIVE
static void vpx_destroy() {
	if (vpx_codec_destroy(&codec))
		printf("Failed to destroy codec.\n");
}

EMSCRIPTEN_KEEPALIVE
static double vpx_decode_frame(const unsigned char *data, size_t data_len) {
	double start = emscripten_get_now();

	if (vpx_codec_decode(&codec, data, (unsigned int)data_len, NULL, 0))
    	printf("Failed to decode frame.\n");
	else {		
		vpx_image_t *img = NULL;
  		vpx_codec_iter_t iter = NULL;

		while ((img = vpx_codec_get_frame(&codec, &iter)) != NULL) {
      		printf("decoded a %d x %d image\n", img->d_w, img->d_h) ;

			//call a JavaScript callback here?

			vpx_img_free(img);
		}
	};

	double elapsed = emscripten_get_now() - start;

	return elapsed ;
	//vpx_codec_decode(&vpxContext, (const uint8_t *)data, data_len, NULL, 1);
	// @todo check return value
	//vpx_codec_decode(&vpxContext, NULL, 0, NULL, 1);
}