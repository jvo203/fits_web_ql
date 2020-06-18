#include <emscripten.h>
#include <emscripten/bind.h>
#include <emscripten/val.h>

#include <iostream>
#include <string>
#include <vector>

#include <fpzip/include/fpzip.h>

using namespace emscripten;

std::vector<float> FPunzip(std::string const &bytes)
{
  std::cout << "[fpunzip] " << bytes.size() << " bytes." << std::endl;

  FPZ *fpz = fpzip_read_from_buffer(bytes.data());

  /* read header */
  if (!fpzip_read_header(fpz))
  {
    fprintf(stderr, "cannot read header: %s\n", fpzip_errstr[fpzip_errno]);
    return std::vector<float>();
  }

  // decompress into <spectrum.data()>
  uint32_t spec_len = fpz->nx;

  if (spec_len == 0)
  {
    fprintf(stderr, "zero-sized fpzip array\n");
    return std::vector<float>();
  }

  std::vector<float> spectrum(spec_len, 0.0f);

  if ((fpz->ny != 1) || (fpz->nz != 1) || (fpz->nf != 1))
  {
    fprintf(stderr, "array size does not match dimensions from header\n");
    return std::vector<float>();
  }

  /* perform actual decompression */
  if (!fpzip_read(fpz, spectrum.data()))
  {
    fprintf(stderr, "decompression failed: %s\n", fpzip_errstr[fpzip_errno]);
    return std::vector<float>();
  }

  fpzip_read_close(fpz);

  return spectrum;
}

EMSCRIPTEN_BINDINGS(Wrapper)
{
  register_vector<float>("std::vector<float>");
  function("FPunzip", &FPunzip);
}
