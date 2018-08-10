#include <vpx/vpx_decoder.h>
#include <vpx/vp8dx.h>

static vpx_codec_ctx_t    vpxContext;
static vpx_codec_iface_t *vpxDecoder = NULL ;

static int vpx_version() {
	return 0;
}

static void vpx_init() {

}

static void vpx_destroy() {

}

static void vpx_decode_frame(const char *data, size_t data_len) {
	vpx_codec_decode(&vpxContext, (const uint8_t *)data, data_len, NULL, 1);
	// @todo check return value
	vpx_codec_decode(&vpxContext, NULL, 0, NULL, 1);
}