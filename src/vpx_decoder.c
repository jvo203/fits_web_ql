#include <vpx/vpx_decoder.h>
#include <vpx/vp8dx.h>

//static vpx_codec_ctx_t    vpxContext;
//static vpx_codec_iface_t *vpxDecoder = NULL ;

static vpx_codec_ctx_t codec;

static int vpx_version() {
	return 0;
}

static void vpx_init() {
	if (vpx_codec_dec_init(&codec, vpx_codec_vp9_dx(), NULL, 0))
    	printf("Failed to initialize decoder.\n");
}

static void vpx_destroy() {
	if (vpx_codec_destroy(&codec))
		printf(&codec, "Failed to destroy codec.\n");
}

static void vpx_decode_frame(const char *data, size_t data_len) {
	if (vpx_codec_decode(&codec, frame, (unsigned int)frame_size, NULL, 0))
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

	//vpx_codec_decode(&vpxContext, (const uint8_t *)data, data_len, NULL, 1);
	// @todo check return value
	//vpx_codec_decode(&vpxContext, NULL, 0, NULL, 1);
}