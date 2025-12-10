use crate::{
    PngResult,
    error::PngError,
    headers::{read_be_u16, read_be_u32},
};

#[derive(Debug, Clone)]
/// Animated PNG frame
pub struct Frame {
    /// Width of the frame
    pub width: u32,
    /// Height of the frame
    pub height: u32,
    /// X offset of the frame
    pub x_offset: u32,
    /// Y offset of the frame
    pub y_offset: u32,
    /// Frame delay numerator
    pub delay_num: u16,
    /// Frame delay denominator
    pub delay_den: u16,
    /// Frame disposal operation
    pub dispose_op: u8,
    /// Frame blend operation
    pub blend_op: u8,
    /// Frame data, from fdAT chunks
    pub data: Vec<u8>,
}

impl Frame {
    /// Construct a new Frame from the data in a fcTL chunk
    pub fn from_fctl_data(byte_data: &[u8]) -> PngResult<Self> {
        if byte_data.len() < 26 {
            return Err(PngError::TruncatedData);
        }
        Ok(Self {
            width: read_be_u32(&byte_data[4..8]),
            height: read_be_u32(&byte_data[8..12]),
            x_offset: read_be_u32(&byte_data[12..16]),
            y_offset: read_be_u32(&byte_data[16..20]),
            delay_num: read_be_u16(&byte_data[20..22]),
            delay_den: read_be_u16(&byte_data[22..24]),
            dispose_op: byte_data[24],
            blend_op: byte_data[25],
            data: vec![],
        })
    }

    /// Construct the data for a fcTL chunk using the given sequence number
    #[must_use]
    pub fn fctl_data(&self, sequence_number: u32) -> Vec<u8> {
        let mut byte_data = Vec::with_capacity(26);
        byte_data.extend_from_slice(&sequence_number.to_be_bytes());
        byte_data.extend_from_slice(&self.width.to_be_bytes());
        byte_data.extend_from_slice(&self.height.to_be_bytes());
        byte_data.extend_from_slice(&self.x_offset.to_be_bytes());
        byte_data.extend_from_slice(&self.y_offset.to_be_bytes());
        byte_data.extend_from_slice(&self.delay_num.to_be_bytes());
        byte_data.extend_from_slice(&self.delay_den.to_be_bytes());
        byte_data.extend_from_slice(&[self.dispose_op, self.blend_op]);
        byte_data
    }

    /// Construct the data for a fdAT chunk using the given sequence number
    #[must_use]
    pub fn fdat_data(&self, sequence_number: u32) -> Vec<u8> {
        let mut byte_data = Vec::with_capacity(4 + self.data.len());
        byte_data.extend_from_slice(&sequence_number.to_be_bytes());
        byte_data.extend_from_slice(&self.data);
        byte_data
    }
}
