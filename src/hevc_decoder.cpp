#include <emscripten.h>

#include <string.h>

#include <fstream>

//colourmaps
#include "colourmap.h"

#include "TLibCommon/TComPicYuv.h"
#include "TLibDecoder/TDecTop.h"
#include "TLibDecoder/AnnexBread.h"
#include "TLibDecoder/NALread.h"

static TDecTop m_cTDecTop;
static Int m_iSkipFrame;
static Int m_iPOCLastDisplay;

static void EMSCRIPTEN_KEEPALIVE hevc_init() asm("hevc_init") ;
static double EMSCRIPTEN_KEEPALIVE hevc_decode_nal_unit(const unsigned char *data, size_t data_len) asm("hevc_decode_nal_unit") ;

void hevc_init()
{  
    // print information
    printf( "\n" );
    printf( "HEVC jctvc-hm reference software decoder version [%s] (including RExt)", NV_VERSION );
    printf( NVM_ONOS );
    printf( NVM_COMPILEDBY );
    printf( NVM_BITS );
    printf( "\n" );

    // create decoder class
    m_cTDecTop.create();

    // initialize decoder class
    m_cTDecTop.init();

    m_iSkipFrame = 0;
    m_iPOCLastDisplay = 0;
}

double hevc_decode_nal_unit(const unsigned char *data, size_t data_len)
{
    double start = emscripten_get_now();
    double stop = 0.0 ;

    std::fstream bitstream;
    bitstream.rdbuf()->pubsetbuf((char*)data, data_len);

    InputByteStream bytestream(bitstream);    

    //while (!!bitstream)//no need for a loop, we are passing only one NAL unit anyway

    InputNALUnit nalu;

    //AnnexBStats stats = AnnexBStats();
    //byteStreamNALUnit(bytestream, nalu.getBitstream().getFifo(), stats);

    vector<uint8_t>& nalUnit = nalu.getBitstream().getFifo();

    nalUnit.insert(nalUnit.end(), data, data+data_len);

    if (nalu.getBitstream().getFifo().empty()) {
        printf("Warning: Attempt to decode an empty NAL unit\n");
    }
    else {
        //read(nalu);
        readNalUnitHeader(nalu);

        //decode it        
        bool has_picture = m_cTDecTop.decode(nalu, m_iSkipFrame, m_iPOCLastDisplay);

        printf("HEVC: decoded a NAL unit, has_picture = %s\n", has_picture ? "true" : "false");
    }

    stop = emscripten_get_now();

    printf("[wasm hevc] elapsed time %5.2f [ms]\n", (stop-start)) ;

    double elapsed = stop - start;

	return elapsed ;
}