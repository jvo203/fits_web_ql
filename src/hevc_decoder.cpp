#include <emscripten.h>

#include <string.h>

//colourmaps
#include "colourmap.h"

#include "TLibCommon/TComPicYuv.h"
#include "TLibDecoder/TDecTop.h"

static TDecTop m_cTDecTop;

EMSCRIPTEN_KEEPALIVE
static void hevc_init() {  
    // create decoder class
    m_cTDecTop.create();

    // initialize decoder class
    m_cTDecTop.init();    
}