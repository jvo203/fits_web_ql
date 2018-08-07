typedef int16 int16_t;
typedef int32 int32_t;
typedef int64 int64_t;
typedef unsigned int16 uint16_t;
typedef unsigned int32 uint32_t;
typedef unsigned int64 uint64_t;

bool isinf(float tmp)
{
  unsigned int i = intbits(tmp) ;
  
  return ( (i & 0x7fffffff) == 0x7f800000 ) || ( (i & 0xffffffff) == 0xff800000 ) ;
}

export uniform float calculate_radial_spectrumF16(uniform int16 cubeData[], uniform float bzero, uniform float bscale, uniform float datamin, uniform float datamax, uniform unsigned int width, uniform int x1, uniform int x2, uniform int y1, uniform int y2, uniform int cx, uniform int cy, uniform int r2, uniform bool average, uniform float cdelt3)
{  
  float partial_sum = 0.0f ;
  unsigned int64 partial_count = 0 ;
  
  foreach (y=y1 ... y2, x=x1 ... x2)
    {      
      float tmp = bzero + half_to_float_fast(cubeData[y*width+x]) * bscale ;
      bool nan = isnan(tmp) || isinf(tmp) || (tmp < datamin) || (tmp > datamax) ;

      float dist2 = (cx-x)*(cx-x) + (cy-y)*(cy-y) ;
      bool exclude = (dist2 > r2) ;
      
      partial_sum += (nan || exclude) ? 0.0f : tmp ;
      partial_count += (nan || exclude) ? 0 : 1 ;
    } ;

  uniform float sum = reduce_add(partial_sum) ;
  uniform int64 count = reduce_add(partial_count) ;

  if(count > 0)
    {
      if(average)
	return sum / (float) count ;
      else
	return sum * cdelt3 ;
    }
  else
    return 0.0f ;
} ;