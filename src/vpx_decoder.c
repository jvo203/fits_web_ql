#include <emscripten.h>

#include <vpx/vpx_decoder.h>
#include <vpx/vp8dx.h>

#include <string.h>

//colourmaps
#include "colourmap.h"

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
static double vpx_decode_frame(const unsigned char *data, size_t data_len, unsigned char* canvas, unsigned int _w, unsigned int _h, const char* colourmap) {
	double start = emscripten_get_now();
	double stop = 0.0 ;

	if (vpx_codec_decode(&codec, data, (unsigned int)data_len, NULL, 0))
    	printf("Failed to decode frame.\n");
	else {		
		vpx_image_t *img = NULL;
  		vpx_codec_iter_t iter = NULL;

		while ((img = vpx_codec_get_frame(&codec, &iter)) != NULL) {
			stop = emscripten_get_now();

      		printf("[wasm] decoded a %d x %d image, elapsed time %5.2f [ms], applying %s colourmap\n", img->d_w, img->d_h, (stop-start), colourmap) ;

			//fill-in the canvas data here
			int w = img->d_w ;
			int h = img->d_h ;
			int stride = img->stride[0] ;
			const unsigned char* luma = img->planes[0] ;

			if(w == _w && h == _h)
			{								
				//apply a colourmap
				if(strcmp(colourmap, "red") == 0)
				{
					apply_colourmap(canvas, luma, w, h, stride, false, ocean_g, ocean_r, ocean_b);
				}
				else if(strcmp(colourmap, "green") == 0)
				{
					apply_colourmap(canvas, luma, w, h, stride, false, ocean_r, ocean_g, ocean_b);
				}
				else if(strcmp(colourmap, "blue") == 0)
				{
					apply_colourmap(canvas, luma, w, h, stride, false, ocean_b, ocean_r, ocean_g);
				}
				else if(strcmp(colourmap, "hot") == 0)
				{
					apply_colourmap(canvas, luma, w, h, stride, false, hot_r, hot_g, hot_b);
				}
				else if(strcmp(colourmap, "haxby") == 0)
				{
					apply_colourmap(canvas, luma, w, h, stride, false, haxby_r, haxby_g, haxby_b);
				}
				else if(strcmp(colourmap, "rainbow") == 0)
				{					
					apply_colourmap(canvas, luma, w, h, stride, true, rainbow_r, rainbow_g, rainbow_b);
				}
				else if(strcmp(colourmap, "cubehelix") == 0)
				{
					apply_colourmap(canvas, luma, w, h, stride, false, cubehelix_r, cubehelix_g, cubehelix_b);
				}
				else if(strcmp(colourmap, "parula") == 0)
				{
					apply_colourmap(canvas, luma, w, h, stride, false, parula_r, parula_g, parula_b);
				}
				else if(strcmp(colourmap, "inferno") == 0)
				{
					apply_colourmap(canvas, luma, w, h, stride, false, inferno_r, inferno_g, inferno_b);
				}
				else if(strcmp(colourmap, "magma") == 0)
				{
					apply_colourmap(canvas, luma, w, h, stride, false, magma_r, magma_g, magma_b);
				}
				else if(strcmp(colourmap, "plasma") == 0)
				{
					apply_colourmap(canvas, luma, w, h, stride, false, plasma_r, plasma_g, plasma_b);
				}
				else if(strcmp(colourmap, "viridis") == 0)
				{
					apply_colourmap(canvas, luma, w, h, stride, false, viridis_r, viridis_g, viridis_b);
				}
				else
				{
					//no colour by default
					apply_greyscale(canvas, luma, w, h, stride);
				};
			}
			else
				printf("[wasm] canvas image dimensions %d x %d do not match the decoded image size, doing nothing\n", _w, _h);

			vpx_img_free(img);
		}
	};

	double elapsed = stop - start;

	return elapsed ;
	//vpx_codec_decode(&vpxContext, (const uint8_t *)data, data_len, NULL, 1);
	// @todo check return value
	//vpx_codec_decode(&vpxContext, NULL, 0, NULL, 1);
}