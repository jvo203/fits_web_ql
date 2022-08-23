typedef int8 int8_t;
typedef int16 int16_t;
typedef int32 int32_t;
typedef int64 int64_t;
typedef unsigned int8 uint8_t;
typedef unsigned int16 uint16_t;
typedef unsigned int32 uint32_t;
typedef unsigned int64 uint64_t;

bool isinf(float tmp)
{
  unsigned int i = intbits(tmp) ;
  
  return ( (i & 0x7fffffff) == 0x7f800000 ) || ( (i & 0xffffffff) == 0xff800000 ) ;
}

uint32_t bswap32(uint32_t u)
{
  return ((u&0xFF)<<24) | (((u>>8)&0xFF)<<16) | (((u>>16)&0xFF)<<8) | (((u>>24)&0xFF)<<0);
}

export void revert_image_u8(uniform unsigned int8 image[], uniform int32 width, uniform int32 height)
{  
  foreach(j = 0 ... height / 2, i = 0 ... width)
  {
    float tmp = image[j * width + i];
    image[j * width + i] = image[(height - 1 - j) * width + i];
    image[(height - 1 - j) * width + i] = tmp;    
  }
}

export uniform float pixels_mean_brightness_ratio(uniform float pixels[], uniform unsigned int8 mask[], uniform float black, uniform float sensitivity, uniform unsigned int total_size)
{
  float partial_sum = 0.0f ;
  unsigned int partial_count = 0 ;

  foreach(j=0 ... total_size)
  {
    cif(mask[j] > 0)
    {      
      float pixel = 5.0f * (pixels[j] - black) * sensitivity ;
      partial_sum += pixel > 0.0f ? pixel/(1.0f + pixel) : 0.0f ;       
      partial_count += 1 ;
    }   
  }

  uniform float sum = reduce_add(partial_sum) ;
  uniform int count = reduce_add(partial_count) ;

  if(count > 0)
    return sum / (float) count ;
  else
    return 0.0f;
} ;

export void make_image_spectrumF16_minmax(uniform int16 fitsData[], uniform float bzero, uniform float bscale, uniform float ignrval, uniform float datamin, uniform float datamax, uniform float cdelt3, uniform float pixels[], uniform unsigned int8 mask[], uniform unsigned int total_size, uniform float references[])
{
  uniform float frame_min = references[0];
  uniform float frame_max = references[1];
  uniform double mean = references[2];
  uniform double integrated = references[3];

  double partial_sum = 0.0 ;
  unsigned int64 partial_count = 0 ;

  float partial_min = frame_min ;
  float partial_max = frame_max ;

  foreach(j=0 ... total_size)
    {
      float tmp = bzero + half_to_float_fast(fitsData[j]) * bscale ;
      bool nan = isnan(tmp) || isinf(tmp) || (tmp < datamin) || (tmp > datamax) || (tmp <= ignrval) ;
      
      /*pixels[j] += nan ? 0.0f : tmp ;
      mask[j] = nan ? mask[j] : 255 ;
      partial_sum += nan ? 0.0f : tmp ;
      partial_count += nan ? 0 : 1 ;
      partial_min = nan ? partial_min : min(partial_min, tmp);
      partial_max = nan ? partial_max : max(partial_max, tmp);*/

      //alternative coherent if
      cif (!nan)
      {
        pixels[j] += tmp ;
        mask[j] = 255 ;
        partial_sum += tmp ;
        partial_count += 1 ;
        partial_min = min(partial_min, tmp);
        partial_max = max(partial_max, tmp);
      }
    } ;

  uniform double sum = reduce_add(partial_sum) ;
  uniform int64 count = reduce_add(partial_count) ;

  if(count > 0)
    {
      mean = (double)(sum / (double) count) ;
      integrated = (double)(sum * cdelt3) ;
    }
  else
    {
      mean = 0.0 ;
      integrated = 0.0 ;
    }

  frame_min = reduce_min(partial_min);
  frame_max = reduce_max(partial_max);

  references[0] = frame_min ;
  references[1] = frame_max ;
  references[2] = (float)mean ;
  references[3] = (float)integrated ;
} ;

export void make_image_spectrumU8_minmax(uniform uint8_t fitsData[], uniform float bzero, uniform float bscale, uniform float ignrval, uniform float datamin, uniform float datamax, uniform float cdelt3, uniform float pixels[], uniform unsigned int8 mask[], uniform unsigned int total_size, uniform float references[])
{
  uniform float frame_min = references[0];
  uniform float frame_max = references[1];
  uniform double mean = references[2];
  uniform double integrated = references[3];

  double partial_sum = 0.0 ;
  unsigned int64 partial_count = 0 ;

  float partial_min = frame_min ;
  float partial_max = frame_max ;

  foreach(j=0 ... total_size)
    {
      float tmp = bzero + (float)fitsData[j] * bscale ;//uint8 converted to float
      bool nan = isnan(tmp) || isinf(tmp) || (tmp < datamin) || (tmp > datamax) || (tmp <= ignrval) ;

      //alternative coherent if
      cif (!nan)
      {
        pixels[j] += tmp ;
        mask[j] = 255 ;
        partial_sum += tmp ;
        partial_count += 1 ;
        partial_min = min(partial_min, tmp);
        partial_max = max(partial_max, tmp);
      }
    } ;

  uniform double sum = reduce_add(partial_sum) ;
  uniform int64 count = reduce_add(partial_count) ;

  if(count > 0)
    {
      mean = (double)(sum / (double) count) ;
      integrated = (double)(sum * cdelt3) ;
    }
  else
    {
      mean = 0.0 ;
      integrated = 0.0 ;
    }

  frame_min = reduce_min(partial_min);
  frame_max = reduce_max(partial_max);

  references[0] = frame_min ;
  references[1] = frame_max ;
  references[2] = (float)mean ;
  references[3] = (float)integrated ;
} ;

export void make_image_spectrumI16_minmax(uniform int16_t fitsData[], uniform float bzero, uniform float bscale, uniform float ignrval, uniform float datamin, uniform float datamax, uniform float cdelt3, uniform float pixels[], uniform unsigned int8 mask[], uniform unsigned int total_size, uniform float references[])
{
  uniform float frame_min = references[0];
  uniform float frame_max = references[1];
  uniform double mean = references[2];
  uniform double integrated = references[3];

  double partial_sum = 0.0 ;
  unsigned int64 partial_count = 0 ;

  float partial_min = frame_min ;
  float partial_max = frame_max ;

  foreach(j=0 ... total_size)
    {
      float tmp = bzero + (float)fitsData[j] * bscale ;//int16 converted to float
      bool nan = isnan(tmp) || isinf(tmp) || (tmp < datamin) || (tmp > datamax) || (tmp <= ignrval) ;

      //alternative coherent if
      cif (!nan)
      {
        pixels[j] += tmp ;
        mask[j] = 255 ;
        partial_sum += tmp ;
        partial_count += 1 ;
        partial_min = min(partial_min, tmp);
        partial_max = max(partial_max, tmp);
      }
    } ;

  uniform double sum = reduce_add(partial_sum) ;
  uniform int64 count = reduce_add(partial_count) ;

  if(count > 0)
    {
      mean = (double)(sum / (double) count) ;
      integrated = (double)(sum * cdelt3) ;
    }
  else
    {
      mean = 0.0 ;
      integrated = 0.0 ;
    }

  frame_min = reduce_min(partial_min);
  frame_max = reduce_max(partial_max);

  references[0] = frame_min ;
  references[1] = frame_max ;
  references[2] = (float)mean ;
  references[3] = (float)integrated ;
} ;

export void make_image_spectrumI32_minmax(uniform int32_t fitsData[], uniform float bzero, uniform float bscale, uniform float ignrval, uniform float datamin, uniform float datamax, uniform float cdelt3, uniform float pixels[], uniform unsigned int8 mask[], uniform unsigned int total_size, uniform float references[])
{
  uniform float frame_min = references[0];
  uniform float frame_max = references[1];
  uniform double mean = references[2];
  uniform double integrated = references[3];

  double partial_sum = 0.0 ;
  unsigned int64 partial_count = 0 ;

  float partial_min = frame_min ;
  float partial_max = frame_max ;

  foreach(j=0 ... total_size)
    {
      float tmp = bzero + (float)fitsData[j] * bscale ;//int32 converted to float
      bool nan = isnan(tmp) || isinf(tmp) || (tmp < datamin) || (tmp > datamax) || (tmp <= ignrval) ;

      //alternative coherent if
      cif (!nan)
      {
        pixels[j] += tmp ;
        mask[j] = 255 ;
        partial_sum += tmp ;
        partial_count += 1 ;
        partial_min = min(partial_min, tmp);
        partial_max = max(partial_max, tmp);
      }
    } ;

  uniform double sum = reduce_add(partial_sum) ;
  uniform int64 count = reduce_add(partial_count) ;

  if(count > 0)
    {
      mean = (double)(sum / (double) count) ;
      integrated = (double)(sum * cdelt3) ;
    }
  else
    {
      mean = 0.0 ;
      integrated = 0.0 ;
    }

  frame_min = reduce_min(partial_min);
  frame_max = reduce_max(partial_max);

  references[0] = frame_min ;
  references[1] = frame_max ;
  references[2] = (float)mean ;
  references[3] = (float)integrated ;
} ;

export void make_image_spectrumF64_minmax(uniform double fitsData[], uniform float bzero, uniform float bscale, uniform float ignrval, uniform float datamin, uniform float datamax, uniform float cdelt3, uniform float pixels[], uniform unsigned int8 mask[], uniform unsigned int total_size, uniform float references[])
{
  uniform float frame_min = references[0];
  uniform float frame_max = references[1];
  uniform double mean = references[2];
  uniform double integrated = references[3];

  double partial_sum = 0.0 ;
  unsigned int64 partial_count = 0 ;

  float partial_min = frame_min ;
  float partial_max = frame_max ;

  foreach(j=0 ... total_size)
    {
      float tmp = bzero + fitsData[j] * bscale ;//double converted to float
      bool nan = isnan(tmp) || isinf(tmp) || (tmp < datamin) || (tmp > datamax) || (tmp <= ignrval) ;
      
      /*pixels[j] += nan ? 0.0f : tmp ;
      mask[j] = nan ? mask[j] : 255 ;
      partial_sum += nan ? 0.0f : tmp ;
      partial_count += nan ? 0 : 1 ;
      partial_min = nan ? partial_min : min(partial_min, tmp);
      partial_max = nan ? partial_max : max(partial_max, tmp);*/

      //alternative coherent if
      cif (!nan)
      {
        pixels[j] += tmp ;
        mask[j] = 255 ;
        partial_sum += tmp ;
        partial_count += 1 ;
        partial_min = min(partial_min, tmp);
        partial_max = max(partial_max, tmp);
      }
    } ;

  uniform double sum = reduce_add(partial_sum) ;
  uniform int64 count = reduce_add(partial_count) ;

  if(count > 0)
    {
      mean = (double)(sum / (double) count) ;
      integrated = (double)(sum * cdelt3) ;
    }
  else
    {
      mean = 0.0 ;
      integrated = 0.0 ;
    }

  frame_min = reduce_min(partial_min);
  frame_max = reduce_max(partial_max);

  references[0] = frame_min ;
  references[1] = frame_max ;
  references[2] = (float)mean ;
  references[3] = (float)integrated ;
} ;

export void make_image_spectrumF32_minmax(uniform int32 src[], uniform int16 dest[], uniform float bzero, uniform float bscale, uniform float ignrval, uniform float datamin, uniform float datamax, uniform float cdelt3, uniform float pixels[], uniform unsigned int8 mask[], uniform unsigned int total_size, uniform float references[])
{
  uniform float frame_min = references[0];
  uniform float frame_max = references[1];
  uniform double mean = references[2];
  uniform double integrated = references[3];

  double partial_sum = 0.0 ;
  unsigned int64 partial_count = 0 ;

  float partial_min = frame_min ;
  float partial_max = frame_max ;

  foreach(j=0 ... total_size)
    {
      float tmp = floatbits(bswap32(src[j])) ;
      dest[j] = float_to_half_fast(tmp) ;

      tmp = bzero + tmp * bscale ;
      bool nan = isnan(tmp) || isinf(tmp) || (tmp < datamin) || (tmp > datamax) || (tmp <= ignrval) ;
      
      /*pixels[j] += nan ? 0.0f : tmp ;
      mask[j] = nan ? mask[j] : 255 ;
      partial_sum += nan ? 0.0f : tmp ;
      partial_count += nan ? 0 : 1 ;
      partial_min = nan ? partial_min : min(partial_min, tmp);
      partial_max = nan ? partial_max : max(partial_max, tmp);*/

      //alternative coherent if
      cif (!nan)
      {
        pixels[j] += tmp ;
        mask[j] = 255 ;
        partial_sum += tmp ;
        partial_count += 1 ;
        partial_min = min(partial_min, tmp);
        partial_max = max(partial_max, tmp);
      }
    } ;

  uniform double sum = reduce_add(partial_sum) ;
  uniform int64 count = reduce_add(partial_count) ;

  if(count > 0)
    {
      mean = (double)(sum / (double) count) ;
      integrated = (double)(sum * cdelt3) ;
    }
  else
    {
      mean = 0.0 ;
      integrated = 0.0 ;
    }

  frame_min = reduce_min(partial_min);
  frame_max = reduce_max(partial_max);

  references[0] = frame_min ;
  references[1] = frame_max ;
  references[2] = (float)mean ;
  references[3] = (float)integrated ;
} ;

export void make_image_spectrumF32_2_F16(uniform float src[], uniform unsigned int8 src_mask[], uniform float frame_min, uniform float frame_max, uniform int16 dest[], uniform float bzero, uniform float bscale, uniform float cdelt3, uniform float pixels[], uniform unsigned int8 dst_mask[], uniform unsigned int total_size, uniform float references[])
{  
  uniform double mean = references[0];
  uniform double integrated = references[1];

  double partial_sum = 0.0 ;
  unsigned int64 partial_count = 0 ;

  foreach(j=0 ... total_size)
    {
      bool nan = (src_mask[j] == 0) ? true : false ;
      float tmp = src[j] ;
      //tmp = frame_min + (exp(tmp) - 0.5f) * (frame_max - frame_min);

      //transform tmp, set it to NaN if nan      
      int iflt_nan = 0x7ff80000; // quiet NaN             
      tmp = nan ? floatbits(iflt_nan) : tmp;
      dest[j] = float_to_half_fast(tmp) ;
      dst_mask[j] = src_mask[j];

      tmp = bzero + tmp * bscale ;

      //alternative coherent if
      cif (!nan)
      {
        pixels[j] += tmp ;        
        partial_sum += tmp ;
        partial_count += 1 ;        
      }
    } ;

  uniform double sum = reduce_add(partial_sum) ;
  uniform int64 count = reduce_add(partial_count) ;

  if(count > 0)
    {
      mean = (double)(sum / (double) count) ;
      integrated = (double)(sum * cdelt3) ;
    }
  else
    {
      mean = 0.0 ;
      integrated = 0.0 ;
    }
  
  references[0] = (float)mean ;
  references[1] = (float)integrated ;
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
  double partial_sum = 0.0 ;
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

  uniform double sum = reduce_add(partial_sum) ;
  uniform int64 count = reduce_add(partial_count) ;

  if(count > 0)
    {
      if(average)
	      return (float)(sum / (double) count) ;
      else
	      return (float)(sum * cdelt3) ;
    }
  else
    return 0.0f ;
} ;

export uniform float calculate_square_spectrumF16(uniform int16 cubeData[], uniform float bzero, uniform float bscale, uniform float datamin, uniform float datamax, uniform unsigned int width, uniform int x1, uniform int x2, uniform int y1, uniform int y2, uniform bool average, uniform float cdelt3)
{
  double partial_sum = 0.0 ;
  unsigned int64 partial_count = 0 ;
  
  foreach (y=y1 ... y2, x=x1 ... x2)
    {      
      float tmp = bzero + half_to_float_fast(cubeData[y*width+x]) * bscale ;
      bool nan = isnan(tmp) || isinf(tmp) || (tmp < datamin) || (tmp > datamax) ;
      
      partial_sum += nan ? 0.0f : tmp ;
      partial_count += nan ? 0 : 1 ;
    } ;

  uniform double sum = reduce_add(partial_sum) ;
  uniform int64 count = reduce_add(partial_count) ;

  if(count > 0)
    {
      if(average)
	      return (float)(sum / (double) count) ;
      else
	      return (float)(sum * cdelt3) ;
    }
  else
    return 0.0f ;
} ;

export void data_to_luminance_f16_linear(uniform int16 cubeData[], uniform unsigned int8 mask[], uniform float bzero, uniform float bscale, uniform float black, uniform float slope, uniform unsigned int8 pixels[],  uniform unsigned int total_size)
{
  foreach(j=0 ... total_size)
  {
    cif(mask[j] > 0)
    {
      float tmp = bzero + half_to_float_fast(cubeData[j]) * bscale ;
      float pixel = 255.0f / ( 1.0f + exp(-6.0f * (tmp - black) * slope) ) ;
      pixels[j] = isnan(tmp) ? 0 : clamp((int) pixel, 0, 255) ;
    }
    else
      pixels[j] = 0 ;
  }
} ;

export void data_to_luminance_f16_logistic(uniform int16 cubeData[], uniform unsigned int8 mask[], uniform float bzero, uniform float bscale, uniform float median, uniform float sensitivity, uniform unsigned int8 pixels[],  uniform unsigned int total_size)
{
  foreach(j=0 ... total_size)
  {
    cif(mask[j] > 0)
    {
      float tmp = bzero + half_to_float_fast(cubeData[j]) * bscale ;
      float pixel = 255.0f / ( 1.0f + exp(-6.0f * (tmp - median) * sensitivity) ) ;
      pixels[j] = clamp( (int) pixel, 0, 255) ;
    }
    else
      pixels[j] = 0 ;
  }
} ;

export void data_to_luminance_f16_ratio(uniform int16 cubeData[], uniform unsigned int8 mask[], uniform float bzero, uniform float bscale, uniform float black, uniform float sensitivity, uniform unsigned int8 pixels[],  uniform unsigned int total_size)
{
  foreach(j=0 ... total_size)
  {
    cif(mask[j] > 0)
    {
      float tmp = bzero + half_to_float_fast(cubeData[j]) * bscale ;
      float pixel = 5.0f * (tmp - black) * sensitivity ;
      pixels[j] = pixel > 0.0f ? (int) (255.0f*pixel/(1.0f + pixel)) : 0 ;
    }
    else
      pixels[j] = 0 ;
  }
} ;

export void data_to_luminance_f16_square(uniform int16 cubeData[], uniform unsigned int8 mask[], uniform float bzero, uniform float bscale, uniform float black, uniform float sensitivity, uniform unsigned int8 pixels[],  uniform unsigned int total_size)
{
  foreach(j=0 ... total_size)
  {
    cif(mask[j] > 0)
    {
      float tmp = bzero + half_to_float_fast(cubeData[j]) * bscale ;
      float pixel = (tmp - black) * sensitivity ;
      pixels[j] = pixel > 0.0f ? clamp( (int) (255.0f*pixel*pixel), 0, 255) : 0 ;
    }
    else
      pixels[j] = 0 ;
  }
} ;

export void data_to_luminance_f16_legacy(uniform int16 cubeData[], uniform unsigned int8 mask[], uniform float bzero, uniform float bscale, uniform float dmin, uniform float dmax, uniform float lmin, uniform float lmax, uniform unsigned int8 pixels[],  uniform unsigned int total_size)
{
  foreach(j=0 ... total_size)
  {
    cif(mask[j] > 0)
    {
      float tmp = bzero + half_to_float_fast(cubeData[j]) * bscale ;
      float pixel = 0.5f + (tmp - dmin) / (dmax - dmin) ;
      pixels[j] = pixel > 0.0f ? clamp( (int) (255.0f * (log(pixel) - lmin) / (lmax - lmin)), 0, 255) : 0 ;
    }
    else
      pixels[j] = 0 ;
  }
} ;

export void bilinear_resize(uniform int8 src[], uniform int src_len, uniform int8 dst[], uniform int dst_len, uniform int src_width, uniform int src_height, uniform unsigned int dst_width, uniform unsigned int dst_height, uniform float scale_factor, uniform float filter_width)
{
  foreach(dst_y=0 ... dst_height, dst_x=0 ... dst_width)
  {
    float pixel = 0.0f ;
    float accum = 0.0f ;

    float orig_x = scale_factor * dst_x ;
    float orig_y = scale_factor * dst_y ;

    for(float dx = -filter_width; dx <= filter_width; dx += 1)//use epsilon test!!!
      for(float dy = -filter_width; dy <= filter_width; dy += 1)//use epsilon test!!!
      {
        float coeff_x = 1.0f - abs(dx)/scale_factor;
        float coeff_y = 1.0f - abs(dy)/scale_factor;
        float coeff = coeff_x * coeff_y ;

        float src_x = orig_x + dx ;
        float src_y = orig_y + dy ;

        int src_index = clamp((int)round(src_y), 0, src_height-1) * src_width + clamp( (int)round(src_x), 0, src_width-1) ;
        src_index = clamp(src_index, 0, src_len-1);

        pixel += src[src_index] * coeff ;
        accum += coeff ;
      }

    int dst_index = dst_y * dst_width + dst_x ;
    dst_index = clamp(dst_index, 0, dst_len-1);
    dst[dst_index] = clamp( (int)round(pixel / accum), 0, 255) ;
  }
} ;

export void rgb_to_yuv(uniform unsigned int8 _r[], uniform unsigned int8 _g[], uniform unsigned int8 _b[], uniform unsigned int total_size)
{
  foreach(j=0 ... total_size)
  {
    float r = _r[j];
    float g = _g[j];
    float b = _b[j];

    //ITU-R
    float Y = 0.299f * r + 0.587f * g + 0.114f * b;
    float Cb = -0.169f * r - 0.331f * g + 0.499f * b + 128.0f;
    float Cr = 0.499f * r - 0.418f * g - 0.0813 * b + 128f;

    _r[j] = clamp((int)Y, 0, 255);
    _g[j] = clamp((int)Cb, 0, 255);
    _b[j] = clamp((int)Cr, 0, 255);
  }
}