use std::{mem,ptr};
use vpx_sys::*;
use num_rational::Rational64;


#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TimeInfo {
    pub pts: Option<i64>,
    pub dts: Option<i64>,
    pub duration: Option<u64>,
    pub timebase: Option<Rational64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PSNR {
    pub samples: [u32; 4],
    pub sse: [u64; 4],
    pub psnr: [f64; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Packet {
    pub data : Vec<u8>,
    pub pos : Option<usize>,
    pub stream_index : isize,
    pub t: TimeInfo,

    // side_data : SideData;

    pub is_key: bool,
    pub is_corrupted: bool,
}

impl Packet {
    pub fn with_capacity(capacity: usize) -> Self {
        Packet {
            data : Vec::with_capacity(capacity),
            t: TimeInfo::default(),
            pos : None,
            stream_index : -1,
            is_key: false,
            is_corrupted: false,
        }
    }

    pub fn new() -> Self {
        Self::with_capacity(0)
    }
}

pub struct VP9EncoderConfig {
    pub cfg: vpx_codec_enc_cfg,
}

unsafe impl Send for VP9EncoderConfig {} // TODO: Make sure it cannot be abused

/// VP9 Encoder setup facility
impl VP9EncoderConfig {
    /// Create a new default configuration
    pub fn new() -> Result<VP9EncoderConfig, vpx_codec_err_t> {
        let mut cfg = unsafe { mem::uninitialized() };
        let ret = unsafe { vpx_codec_enc_config_default(vpx_codec_vp9_cx(), &mut cfg, 0) };

        match ret {
            VPX_CODEC_OK => Ok(VP9EncoderConfig { cfg: cfg }),
            _ => Err(ret),
        }
    }

    /// Return a newly allocated `VP9Encoder` using the current configuration
    pub fn get_encoder(&mut self) -> Result<VP9Encoder, vpx_codec_err_t> {
        VP9Encoder::new(self)
    }
}

/// VP9 Encoder
pub struct VP9Encoder {
    pub(crate) ctx: vpx_codec_ctx_t,
    pub(crate) iter: vpx_codec_iter_t,
}

unsafe impl Send for VP9Encoder {} // TODO: Make sure it cannot be abused

impl VP9Encoder {
    /// Create a new encoder using the provided configuration
    ///
    /// You may use `get_encoder` instead.
    pub fn new(cfg: &mut VP9EncoderConfig) -> Result<VP9Encoder, vpx_codec_err_t> {
        let mut ctx = unsafe { mem::uninitialized() };
        let ret = unsafe {
            vpx_codec_enc_init_ver(
                &mut ctx,
                vpx_codec_vp9_cx(),
                &mut cfg.cfg,
                0,
                VPX_ENCODER_ABI_VERSION as i32,
            )
        };

        match ret {
            VPX_CODEC_OK => Ok(VP9Encoder {
                ctx: ctx,
                iter: ptr::null(),
            }),
            _ => Err(ret),
        }
    }

    /// Update the encoder parameters after-creation
    ///
    /// It calls `vpx_codec_control_`
    pub fn control(&mut self, id: vp8e_enc_control_id, val: i32) -> Result<(), vpx_codec_err_t> {
        let ret = unsafe { vpx_codec_control_(&mut self.ctx, id as i32, val) };

        match ret {
            VPX_CODEC_OK => Ok(()),
            _ => Err(ret),
        }
    }

    // TODO: Cache the image information
    //
    /// Send an uncompressed frame to the encoder
    ///
    /// Call [`get_packet`] to receive the compressed data.
    ///
    /// It calls `vpx_codec_encode`.
    ///
    /// [`get_packet`]: #method.get_packet
    pub fn encode(&mut self, frame: &Frame) -> Result<(), vpx_codec_err_t> {
        let mut img = img_from_frame(frame);

        let ret = unsafe {
            vpx_codec_encode(
                &mut self.ctx,
                &mut img,
                frame.t.pts.unwrap(),
                1,
                0,
                VPX_DL_GOOD_QUALITY as u64,
            )
        };

        self.iter = ptr::null();

        match ret {
            VPX_CODEC_OK => Ok(()),
            _ => Err(ret),
        }
    }

    /// Notify the encoder that no more data will be sent
    ///
    /// Call [`get_packet`] to receive the compressed data.
    ///
    /// It calls `vpx_codec_encode` with NULL arguments.
    ///
    /// [`get_packet`]: #method.get_packet
    pub fn flush(&mut self) -> Result<(), vpx_codec_err_t> {
        let ret = unsafe {
             vpx_codec_encode(
                &mut self.ctx,
                ptr::null_mut(),
                0,
                1,
                0,
                VPX_DL_GOOD_QUALITY as u64,
            )
        };

        self.iter = ptr::null();

        match ret {
            VPX_CODEC_OK => Ok(()),
            _ => Err(ret),
        }
    }

    /// Retrieve the compressed data
    ///
    /// To be called until it returns `None`.
    ///
    /// It calls `vpx_codec_get_cx_data`.
    pub fn get_packet(&mut self) -> Option<VPXPacket> {
        let pkt = unsafe { vpx_codec_get_cx_data(&mut self.ctx, &mut self.iter) };

        if pkt.is_null() {
            None
        } else {
            Some(VPXPacket::new(unsafe { *pkt }))
        }
    }
}

impl Drop for VP9Encoder {
    fn drop(&mut self) {
        unsafe { vpx_codec_destroy(&mut self.ctx) };
    }
}

impl VPXCodec for VP9Encoder {
    fn get_context<'a>(&'a mut self) -> &'a mut vpx_codec_ctx {
        &mut self.ctx
    }
}