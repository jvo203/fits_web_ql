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

void apply_greyscale(unsigned char* canvas, const unsigned char* luma, int w, int h, int stride)
{
	size_t dst_offset = 0 ;

	for(int j=0;j<h;j++)
	{	  
	  size_t offset = j * stride ;

	  for(int i=0;i<w;i++)
	    {			
			unsigned char pixel = luma[offset++] ;
			
			canvas[dst_offset++] = pixel ;
			canvas[dst_offset++] = pixel ;
			canvas[dst_offset++] = pixel ;
			canvas[dst_offset++] = 255 ;//the alpha channel
		}
	}
}

EMSCRIPTEN_KEEPALIVE
static double vpx_decode_frame(const unsigned char *data, size_t data_len, unsigned char* canvas, unsigned int _w, unsigned int _h) {
	double start = emscripten_get_now();
	double stop = 0.0 ;

	if (vpx_codec_decode(&codec, data, (unsigned int)data_len, NULL, 0))
    	printf("Failed to decode frame.\n");
	else {		
		vpx_image_t *img = NULL;
  		vpx_codec_iter_t iter = NULL;

		while ((img = vpx_codec_get_frame(&codec, &iter)) != NULL) {
			stop = emscripten_get_now();

      		printf("[wasm] decoded a %d x %d image, elapsed time %5.2f [ms]\n", img->d_w, img->d_h, (stop-start)) ;

			//fill-in the canvas data here
			int w = img->d_w ;
			int h = img->d_h ;
			int stride = img->stride[0] ;
			const unsigned char* luma = img->planes[0] ;

			if(w == _w && h == _h)
				apply_greyscale(canvas, luma, w, h, stride);
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