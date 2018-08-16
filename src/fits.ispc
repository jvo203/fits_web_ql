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

export void make_image_spectrumF16_minmax(uniform int16 fitsData[], uniform float bzero, uniform float bscale, uniform float datamin, uniform float datamax, uniform float cdelt3, uniform float pixels[], uniform unsigned int8 mask[], uniform unsigned int total_size, uniform float references[])
{
  uniform float frame_min = references[0];
  uniform float frame_max = references[1];
  uniform float mean = references[2];
  uniform float integrated = references[3];

  float partial_sum = 0.0f ;
  unsigned int64 partial_count = 0 ;

  float partial_min = frame_min ;
  float partial_max = frame_max ;

  foreach(j=0 ... total_size)
    {
      float tmp = bzero + half_to_float_fast(fitsData[j]) * bscale ;
      bool nan = isnan(tmp) || isinf(tmp) || (tmp < datamin) || (tmp > datamax) ;
      
      /*pixels[j] += nan ? 0.0f : tmp ;
      mask[j] = nan ? mask[j] : 1 ;//boolean mask in Rust
      partial_sum += nan ? 0.0f : tmp ;
      partial_count += nan ? 0 : 1 ;
      partial_min = nan ? partial_min : min(partial_min, tmp);
      partial_max = nan ? partial_max : max(partial_max, tmp);*/

      //alternative coherent if
      cif (!nan)
      {
        pixels[j] += tmp ;
        mask[j] = 1 ;//boolean mask in Rust
        partial_sum += tmp ;
        partial_count += 1 ;
        partial_min = min(partial_min, tmp);
        partial_max = max(partial_max, tmp);
      }
    } ;

  uniform float sum = reduce_add(partial_sum) ;
  uniform int64 count = reduce_add(partial_count) ;

  if(count > 0)
    {
      
      mean = sum / (float) count ;
      integrated = sum * cdelt3 ;
    }
  else
    {
      mean = 0.0f ;
      integrated = 0.0f ;
    }

  frame_min = reduce_min(partial_min);
  frame_max = reduce_max(partial_max);

  references[0] = frame_min ;
  references[1] = frame_max ;
  references[2] = mean ;
  references[3] = integrated ;
} ;

export void join_pixels_masks(uniform float pixels[], uniform float pixels_tid[], uniform unsigned int8 mask[], uniform unsigned int8 mask_tid[], uniform float cdelt3, uniform unsigned int total_size)
{
  foreach(i=0 ... total_size)
    {
      pixels[i] += pixels_tid[i] * cdelt3 ;
      mask[i] |= mask_tid[i] ;
    } ;
} ;

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

export uniform float calculate_square_spectrumF16(uniform int16 cubeData[], uniform float bzero, uniform float bscale, uniform float datamin, uniform float datamax, uniform unsigned int width, uniform int x1, uniform int x2, uniform int y1, uniform int y2, uniform bool average, uniform float cdelt3)
{
  float partial_sum = 0.0f ;
  unsigned int64 partial_count = 0 ;
  
  foreach (y=y1 ... y2, x=x1 ... x2)
    {      
      float tmp = bzero + half_to_float_fast(cubeData[y*width+x]) * bscale ;
      bool nan = isnan(tmp) || isinf(tmp) || (tmp < datamin) || (tmp > datamax) ;
      
      partial_sum += nan ? 0.0f : tmp ;
      partial_count += nan ? 0 : 1 ;
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