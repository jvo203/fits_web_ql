simd:
	ispc -g -O3 --target=avx2-i32x16 --opt=fast-math --addressing=32 src/fits.ispc -o fits.o -h fits.h
	ar -cvq libfits.a fits.o
